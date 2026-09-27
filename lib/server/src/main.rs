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
use self::sdk::provider::Provider;
use self::transit::{Transit, TransitConfig};

use std::sync::Arc;

pub type Res<T> = Result<axum::Json<T>>;

struct Server {
  transit: Transit,
}
impl Server {
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
      .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::gtfs_daily_cache));

    // realtime
    let realtime = axum::Router::new()
      .route("/health", get(handlers::health::health))
      .route("/buses", get(handlers::bus::list))
      .route("/stop/arrivals/{provider}/{id}", get(handlers::stop::arrivals))
      .route("/line/buses/{provider}/{id}", get(handlers::line::buses));

    gtfs.merge(schedule).merge(realtime)
  }
}

#[derive(Clone)]
struct State(Arc<Server>);
impl State {
  #[allow(clippy::expect_used)]
  async fn build() -> State {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let rebuild_webhook = std::env::var("REBUILD_WEBHOOK_URL").ok().filter(|u| !u.trim().is_empty());

    let providers: Vec<Arc<Provider>> = vec![
      Arc::new(provider::stm::provider(stm_conf()).expect("stm: invalid config")),
      Arc::new(provider::dnt::metropolitan().expect("dnt: invalid config")),
      Arc::new(provider::dnt::interdepartmental().expect("inter: invalid config")),
      Arc::new(provider::busmatick::codesa(busmatick_conf("CODESA_URL")).expect("codesa: invalid config")),
      Arc::new(provider::busmatick::mdoturismo(busmatick_conf("MDOTURISMO_URL")).expect("mdoturismo: invalid config")),
      Arc::new(provider::busmatick::micro(busmatick_conf("MICRO_URL")).expect("micro: invalid config")),
      Arc::new(provider::busmatick::solantigua(busmatick_conf("SOLANTIGUA_URL")).expect("solantigua: invalid config")),
      Arc::new(provider::busmatick::copay(busmatick_conf("COPAY_URL")).expect("copay: invalid config")),
      Arc::new(
        provider::busmatick::microrivera(busmatick_conf("MICRORIVERA_URL")).expect("microrivera: invalid config"),
      ),
    ];

    let config = TransitConfig { database_url, redis_url, providers, rebuild_webhook };
    let transit = Transit::connect(config).await.expect("failed to start transit");

    State(Arc::new(Server { transit }))
  }
}

fn stm_conf() -> provider::stm::StmConfig {
  let var = |key: &str| std::env::var(key).unwrap_or_default();
  provider::stm::StmConfig {
    api_url: var("IM_API_URL"),
    auth_url: var("IM_AUTH_URL"),
    client_id: var("IM_CLIENT_ID"),
    client_secret: var("IM_CLIENT_SECRET"),
  }
}

fn busmatick_conf(key: &str) -> provider::busmatick::BusmatickConfig {
  provider::busmatick::BusmatickConfig { base: std::env::var(key).unwrap_or_default() }
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
  state.transit.spawn_tasks();

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
