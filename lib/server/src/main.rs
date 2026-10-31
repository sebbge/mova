use mova_planner as planner;
use mova_sdk as sdk;
use mova_transit as transit;

mod provider {
  pub use mova_provider_busmatick as busmatick;
  pub use mova_provider_dnt as dnt;
  pub use mova_provider_stm as stm;
}

mod error;
mod handlers;
mod middleware;

pub use self::error::{Error, Result};
use self::planner::Planner;
use self::sdk::provider::Provider;
use self::transit::{Transit, TransitConfig};

use std::sync::Arc;

pub type Res<T> = Result<axum::Json<T>>;

struct Server {
  transit: Transit,
  planner: Planner,
}
impl Server {
  pub fn router(state: State) -> axum::Router<State> {
    use axum::routing::get;

    // GTFS
    let gtfs = axum::Router::new()
      .route("/stops", get(handlers::stop::list))
      .route("/stops/search", get(handlers::stop::search))
      .route("/stop/{provider}/{id}", get(handlers::stop::stop))
      .route("/routes", get(handlers::routes::list))
      .route("/routes/search", get(handlers::routes::search))
      .route("/route/shape/{provider}/{id}", get(handlers::routes::shape))
      .route("/shape/{provider}/{id}", get(handlers::routes::variant))
      .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::gtfs_cache));

    // GTFS
    let schedule = axum::Router::new()
      .route("/stop/schedule/{provider}/{id}", get(handlers::stop::schedule))
      .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::gtfs_daily_cache));

    // realtime
    let realtime = axum::Router::new()
      .route("/health", get(handlers::health::health))
      .route("/buses", get(handlers::bus::list))
      .route("/stop/arrivals/{provider}/{id}", get(handlers::stop::arrivals))
      .route("/stop/buses/{provider}/{id}", get(handlers::stop::buses))
      .route("/line/buses/{provider}/{id}", get(handlers::bus::line))
      .route("/route/times/{provider}/{id}", get(handlers::routes::times))
      .route("/route/schedule/{provider}/{id}", get(handlers::routes::schedule))
      .route("/plan", get(handlers::plan::plan));

    // OSM
    let places = axum::Router::new()
      .route("/places/search", get(handlers::place::search))
      .route("/places/reverse", get(handlers::place::reverse))
      .route("/place/{id}", get(handlers::place::place));

    gtfs.merge(schedule).merge(realtime).merge(places)
  }
}

#[derive(Clone)]
struct State(Arc<Server>);
impl State {
  async fn build() -> State {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let rebuild_webhook = std::env::var("REBUILD_WEBHOOK_URL").ok().filter(|u| !u.trim().is_empty());

    let config = TransitConfig { database_url, redis_url, providers: Server::providers(), rebuild_webhook };
    let transit = Transit::connect(config).await.expect("failed to start transit");

    let planner = Planner::start(transit.clone());

    State(Arc::new(Server { transit, planner }))
  }
}

impl Server {
  fn providers() -> Vec<Arc<Provider>> {
    use provider::{busmatick, dnt, stm};

    let var = |key: &str| std::env::var(key).unwrap_or_default();
    let stm = stm::StmConfig {
      api_url: var("IM_API_URL"),
      auth_url: var("IM_AUTH_URL"),
      client_id: var("IM_CLIENT_ID"),
      client_secret: var("IM_CLIENT_SECRET"),
    };
    let base = |key: &str| busmatick::BusmatickConfig { base: var(key) };

    [
      ("stm", stm::provider(stm)),
      ("dnt", dnt::metropolitan()),
      ("inter", dnt::interdepartmental()),
      ("codesa", busmatick::codesa(base("CODESA_URL"))),
      ("mdoturismo", busmatick::mdoturismo(base("MDOTURISMO_URL"))),
      ("micro", busmatick::micro(base("MICRO_URL"))),
      ("solantigua", busmatick::solantigua(base("SOLANTIGUA_URL"))),
      ("copay", busmatick::copay(base("COPAY_URL"))),
      ("microrivera", busmatick::microrivera(base("MICRORIVERA_URL"))),
    ]
    .into_iter()
    .map(|(name, provider)| Arc::new(provider.unwrap_or_else(|e| panic!("{name}: invalid config: {e:?}"))))
    .collect()
  }

  fn cors() -> tower_http::cors::CorsLayer {
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
}

struct LogTime;
impl tracing_subscriber::fmt::time::FormatTime for LogTime {
  fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
    write!(w, "{}", chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S"))
  }
}

#[tokio::main]
async fn main() {
  use tower_http::compression::CompressionLayer;
  use tower_http::trace::TraceLayer;

  tracing_subscriber::fmt().with_target(false).with_timer(LogTime).init();

  let state = State::build().await;
  state.transit.spawn_tasks();

  let app = Server::router(state.clone())
    .layer(TraceLayer::new_for_http())
    .layer(CompressionLayer::new())
    .layer(Server::cors())
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
