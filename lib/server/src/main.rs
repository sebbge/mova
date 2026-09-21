use citymob_core as core;
use citymob_stm as stm;

mod cache;
mod domain;
mod helpers;
mod tasks;

use self::cache::Cache;
use self::core::{Error, Result, provider::Provider};
use self::domain::{handlers, middleware};

use std::sync::Arc;

pub type Res<T> = Result<axum::Json<T>>;
pub type Db = sqlx::PgPool;
pub type GtfsVersion = Arc<tokio::sync::RwLock<String>>;

struct Server {
  provider: Arc<dyn Provider>,
  db: Db,
  cache: Cache,
  gtfs_version: GtfsVersion,
  rebuild_webhook: Option<String>,
}
impl Server {
  pub fn router(gtfs_version: GtfsVersion) -> axum::Router<State> {
    use axum::routing::get;

    // GTFS
    let gtfs = axum::Router::new()
      .route("/stops", get(handlers::stop::list))
      .route("/stops/nearby", get(handlers::nearby_stops::nearby))
      .route("/stops/{stop_id}", get(handlers::stop::stop))
      .route("/routes", get(handlers::routes::list))
      .route("/routes/search", get(handlers::routes::search))
      .route("/routes/{line_id}/shape", get(handlers::routes::shape))
      .layer(axum::middleware::from_fn_with_state(gtfs_version.clone(), middleware::gtfs_cache));

    // GTFS
    let schedule = axum::Router::new()
      .route("/stops/{stop_id}/schedule", get(handlers::stop::schedule))
      .layer(axum::middleware::from_fn_with_state(gtfs_version, middleware::gtfs_daily_cache));

    // realtime
    let realtime = axum::Router::new()
      .route("/health", get(handlers::health::health))
      .route("/stops/{stop_id}/arrivals", get(handlers::stop::arrivals))
      .route("/lines/{line_id}", get(handlers::line::buses));

    gtfs.merge(schedule).merge(realtime)
  }

  pub async fn web_rebuild(&self) {
    let Some(url) = self.rebuild_webhook.as_deref() else {
      return;
    };

    match reqwest::Client::new().post(url).send().await {
      Ok(res) if res.status().is_success() => {
        tracing::info!(status = res.status().as_u16(), "[GTFS] web rebuild")
      }
      Ok(res) => tracing::warn!(status = res.status().as_u16(), "[GTFS] webhook non-2xx"),
      Err(e) => tracing::warn!(error = %e, "[GTFS] couldnt rebuild"),
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

    let version: Option<String> = sqlx::query_scalar("SELECT version FROM gtfs_metadata WHERE id = 1")
      .fetch_optional(&postgres)
      .await
      .ok()
      .flatten();
    let gtfs_version = Arc::new(tokio::sync::RwLock::new(version.unwrap_or_else(|| "none".into())));

    let redis = redis::Client::open(redis_url)
      .expect("invalid REDIS_URL")
      .get_multiplexed_async_connection()
      .await
      .expect("failed to connect to Redis");
    let cache = Cache::new(redis, sync);

    #[cfg(debug_assertions)]
    {
      // DEBUG: force cache cold start
      use cache::Flush;
      cache.flush().await;
    }

    let provider: Arc<dyn Provider> = Arc::new(stm::Stm::new().expect("stm: configuracion invalida"));

    State(Arc::new(Server { provider, db: postgres, cache, gtfs_version, rebuild_webhook }))
  }
}

/// Required in release
fn cors_layer() -> tower_http::cors::CorsLayer {
  use axum::http::{HeaderValue, Method};
  use tower_http::cors::CorsLayer;

  let raw = std::env::var("CORS_ORIGINS").unwrap_or_default();
  let origins: Vec<HeaderValue> =
    raw.split(',').map(str::trim).filter(|o| !o.is_empty()).filter_map(|o| o.parse().ok()).collect();

  if origins.is_empty() {
    if cfg!(debug_assertions) {
      tracing::warn!("CORS_ORIGINS sin definir: se permite cualquier origen (solo en debug)");
      return CorsLayer::permissive();
    }
    panic!("CORS_ORIGINS must be set");
  }

  tracing::info!(count = origins.len(), "CORS restringido");
  CorsLayer::new().allow_origin(origins).allow_methods([Method::GET])
}

#[allow(clippy::unwrap_used)]
#[tokio::main]
async fn main() {
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

  let app =
    Server::router(state.gtfs_version.clone()).layer(TraceLayer::new_for_http()).layer(cors_layer()).with_state(state);

  let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
  tracing::info!("listening on {}", listener.local_addr().unwrap());
  axum::serve(listener, app).await.unwrap();
}

impl std::ops::Deref for State {
  type Target = Server;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
