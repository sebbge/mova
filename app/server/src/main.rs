use std::sync::Arc;

mod error;
use error::{Error, Result};

mod helpers;
mod tasks;

mod im_api;
use im_api::ImApi;

mod bondi;
use bondi::handlers;

mod cache;
use cache::Cache;

pub type Res<T> = Result<axum::Json<T>>;
pub type Db = sqlx::PgPool;

struct Server {
  im_api: ImApi,
  db: Db,
  cache: Cache,
}
impl Server {
  pub fn router() -> axum::Router<State> {
    use axum::routing::get;

    axum::Router::new()
      .route("/health", get(handlers::health::health))
      // stops
      .route("/stops/nearby", get(handlers::nearby_stops::nearby))
      .route("/stops/{stop_id}", get(handlers::stop::stop_detail))
      // lines
      .route("/lines/{line_id}", get(handlers::line::buses))
      .route("/routes/{line_id}/shape", get(handlers::routes::shape))
      .route("/routes/search", get(handlers::routes::search))
  }
}

#[derive(Clone)]
struct State(Arc<Server>);
impl State {
  async fn build() -> State {
    dotenvy::dotenv().ok();

    let im_api_url = std::env::var("IM_API_URL").expect("IM_API_URL must be set");
    let im_auth_url = std::env::var("IM_AUTH_URL").expect("IM_AUTH_URL must be set");
    let im_client_id = std::env::var("IM_CLIENT_ID").expect("IM_CLIENT_ID must be set");
    let im_client_secret = std::env::var("IM_CLIENT_SECRET").expect("IM_CLIENT_SECRET must be set");
    let postgres_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");

    let sync = 2 * 3600;

    let postgres = sqlx::postgres::PgPoolOptions::new()
      .max_connections(10)
      .connect(&postgres_url)
      .await
      .expect("failed to connect to database");
    sqlx::migrate!().run(&postgres).await.expect("failed to run migrations");

    let redis = redis::Client::open(redis_url)
      .expect("invalid REDIS_URL")
      .get_multiplexed_tokio_connection()
      .await
      .expect("failed to connect to Redis");
    let cache = Cache::new(redis, sync);

    #[cfg(debug_assertions)]
    {
      // DEBUG: force cache cold start
      use cache::Flush;
      cache.flush().await;
    }

    let im_api = ImApi::new(im_api_url, im_auth_url, im_client_id, im_client_secret);
    State(Arc::new(Server { im_api, db: postgres, cache }))
  }
}

#[tokio::main]
async fn main() {
  use tower_http::{cors::CorsLayer, trace::TraceLayer};

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

  let app = Server::router().layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()).with_state(state);

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
