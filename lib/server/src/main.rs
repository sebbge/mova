use citymob_busmatick as busmatick;
use citymob_core as core;
use citymob_dnt as dnt;
use citymob_stm as stm;

mod cache;
mod domain;
mod error;
mod helpers;
mod tasks;

use self::cache::Cache;
use self::core::provider::Provider;
use self::domain::scope::Scope;
use self::domain::{handlers, middleware};
pub use self::error::{Error, Result};

use std::sync::Arc;

pub type Res<T> = Result<axum::Json<T>>;
pub type Db = sqlx::PgPool;

pub type GtfsVersions = Arc<tokio::sync::RwLock<std::collections::BTreeMap<String, String>>>;

pub fn versions_token(versions: &std::collections::BTreeMap<String, String>) -> String {
  if versions.is_empty() {
    return "none".into();
  }
  versions.iter().map(|(p, v)| format!("{p}={v}")).collect::<Vec<_>>().join(";")
}

struct Server {
  providers: Vec<Arc<Provider>>,
  db: Db,
  tasks_db: Db,
  cache: Cache,
  gtfs_versions: GtfsVersions,
  rebuild_webhook: Option<String>,
}
impl Server {
  pub fn scopes(&self) -> impl Iterator<Item = Scope> + '_ {
    self.providers.iter().cloned().map(Scope::new)
  }

  pub fn handles(&self) -> Vec<Arc<Provider>> {
    self.providers.clone()
  }

  pub fn priority(&self) -> Vec<String> {
    self.providers.iter().map(|p| p.id.to_string()).collect()
  }

  pub fn resolve(&self, provider: &str) -> Result<Scope> {
    let handle = self
      .providers
      .iter()
      .find(|p| p.id == provider)
      .ok_or_else(|| Error::NotFound(format!("unknown provider: {provider}")))?;
    Ok(Scope::new(handle.clone()))
  }

  pub fn router(state: State) -> axum::Router<State> {
    use axum::routing::get;

    // GTFS
    let gtfs = axum::Router::new()
      .route("/stops", get(handlers::stop::list))
      .route("/stop/{provider}/{id}", get(handlers::stop::stop))
      .route("/routes", get(handlers::routes::list))
      .route("/routes/search", get(handlers::routes::search))
      .route("/route/shape/{provider}/{id}", get(handlers::routes::shape))
      .route("/shape/{provider}/{id}", get(handlers::routes::variant))
      .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::gtfs_cache));

    // GTFS
    let schedule = axum::Router::new()
      .route("/stop/schedule/{provider}/{id}", get(handlers::stop::schedule))
      .layer(axum::middleware::from_fn_with_state(state.gtfs_versions.clone(), middleware::gtfs_daily_cache));

    // realtime
    let realtime = axum::Router::new()
      .route("/health", get(handlers::health::health))
      .route("/buses", get(handlers::bus::list))
      .route("/stop/arrivals/{provider}/{id}", get(handlers::stop::arrivals))
      .route("/line/buses/{provider}/{id}", get(handlers::line::buses));

    gtfs.merge(schedule).merge(realtime)
  }

  pub async fn web_rebuild(&self) {
    let Some(url) = self.rebuild_webhook.as_deref() else {
      return;
    };

    match reqwest::Client::new().post(url).send().await {
      Ok(res) if res.status().is_success() => {
        tracing::info!(status = res.status().as_u16(), "[gtfs] web rebuild")
      }
      Ok(res) => tracing::warn!(status = res.status().as_u16(), "[gtfs] webhook non-2xx"),
      Err(e) => tracing::warn!(error = %e, "[gtfs] couldn't rebuild"),
    }
  }
}

#[derive(Clone)]
struct State(Arc<Server>);
impl State {
  #[allow(clippy::expect_used)]
  async fn build() -> State {
    dotenvy::dotenv().ok();

    let postgres_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let rebuild_webhook = std::env::var("REBUILD_WEBHOOK_URL").ok().filter(|u| !u.trim().is_empty());

    let sync = 2 * 3600;

    let postgres = sqlx::postgres::PgPoolOptions::new()
      .max_connections(10)
      .connect(&postgres_url)
      .await
      .expect("failed to connect to database");
    sqlx::migrate!().run(&postgres).await.expect("failed to run migrations");

    let tasks_db = {
      use sqlx::ConnectOptions;
      let options = (*postgres.connect_options())
        .clone()
        .log_slow_statements(tracing::log::LevelFilter::Warn, std::time::Duration::from_secs(30));
      sqlx::postgres::PgPoolOptions::new().max_connections(2).connect_lazy_with(options)
    };

    let versions: Vec<(String, String)> = sqlx::query_as("SELECT provider, version FROM gtfs_metadata")
      .fetch_all(&postgres)
      .await
      .expect("failed to read gtfs versions");
    let gtfs_versions = Arc::new(tokio::sync::RwLock::new(versions.into_iter().collect()));

    let redis = redis::Client::open(redis_url)
      .expect("invalid REDIS_URL")
      .get_multiplexed_async_connection()
      .await
      .expect("failed to connect to Redis");
    let cache = Cache::new(redis, sync);

    let providers: Vec<Arc<Provider>> = vec![
      Arc::new(stm::provider(stm_config()).expect("stm: invalid config")),
      Arc::new(dnt::metropolitan().expect("dnt: invalid config")),
      Arc::new(dnt::interdepartmental().expect("inter: invalid config")),
      Arc::new(busmatick::codesa(busmatick_config("CODESA_URL")).expect("codesa: invalid config")),
      Arc::new(busmatick::mdoturismo(busmatick_config("MDOTURISMO_URL")).expect("mdoturismo: invalid config")),
      Arc::new(busmatick::micro(busmatick_config("MICRO_URL")).expect("micro: invalid config")),
      Arc::new(busmatick::solantigua(busmatick_config("SOLANTIGUA_URL")).expect("solantigua: invalid config")),
    ];

    for provider in &providers {
      tracing::info!(provider = provider.id, authority = %provider.authority, regions = ?provider.regions, "[bootstrap] loaded");

      #[cfg(debug_assertions)]
      {
        // DEBUG: force cache cold start
        use cache::Flush;
        cache.flush_region(provider.authority).await;
      }
    }

    State(Arc::new(Server { providers, db: postgres, tasks_db, cache, gtfs_versions, rebuild_webhook }))
  }
}

fn stm_config() -> stm::StmConfig {
  let var = |key: &str| std::env::var(key).unwrap_or_default();
  stm::StmConfig {
    api_url: var("IM_API_URL"),
    auth_url: var("IM_AUTH_URL"),
    client_id: var("IM_CLIENT_ID"),
    client_secret: var("IM_CLIENT_SECRET"),
  }
}

fn busmatick_config(key: &str) -> busmatick::BusmatickConfig {
  busmatick::BusmatickConfig { base: std::env::var(key).unwrap_or_default() }
}

fn cors_layer() -> tower_http::cors::CorsLayer {
  use axum::http::{HeaderValue, Method};
  use tower_http::cors::CorsLayer;

  if cfg!(debug_assertions) {
    tracing::warn!("[cors] all origins (debug)");
    return CorsLayer::permissive();
  }

  let raw = std::env::var("CORS_ORIGINS").unwrap_or_default();
  let origins: Vec<HeaderValue> =
    raw.split(',').map(str::trim).filter(|o| !o.is_empty()).filter_map(|o| o.parse().ok()).collect();

  if origins.is_empty() {
    panic!("CORS_ORIGINS must be set");
  }

  tracing::info!(count = origins.len(), "[cors] origins restricted");
  CorsLayer::new().allow_origin(origins).allow_methods([Method::GET])
}

#[allow(clippy::unwrap_used)]
#[tokio::main]
async fn main() {
  use tower_http::compression::CompressionLayer;
  use tower_http::trace::TraceLayer;

  {
    use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

    struct Time;
    impl FormatTime for Time {
      fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = chrono::Utc::now();
        write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S"))
      }
    }

    tracing_subscriber::fmt().with_target(false).with_timer(Time).init();
  }

  let state = State::build().await;
  tasks::spawn(&state);

  let app = Server::router(state.clone())
    .layer(TraceLayer::new_for_http())
    .layer(CompressionLayer::new())
    .layer(cors_layer())
    .with_state(state);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  tracing::info!(addr = %listener.local_addr().unwrap(), "[bootstrap] listening");
  axum::serve(listener, app).await.unwrap();
}

impl std::ops::Deref for State {
  type Target = Server;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
