use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod error;
mod gtfs_sync;
mod handlers;
mod im_auth;
mod models;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::PgPool,
    pub redis: redis::aio::MultiplexedConnection,
    pub http_client: reqwest::Client,
    pub im_api_base_url: String,
    pub im_auth: im_auth::ImAuthClient,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let redis_client = redis::Client::open(redis_url).expect("invalid REDIS_URL");
    let redis = redis_client
        .get_multiplexed_tokio_connection()
        .await
        .expect("failed to connect to Redis");

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .expect("failed to build HTTP client");

    let im_auth = im_auth::ImAuthClient::new(
        std::env::var("IM_AUTH_URL").unwrap_or_else(|_| {
            "https://mvdapi-auth.montevideo.gub.uy/auth/realms/pci/protocol/openid-connect/token"
                .into()
        }),
        std::env::var("IM_CLIENT_ID").expect("IM_CLIENT_ID must be set"),
        std::env::var("IM_CLIENT_SECRET").expect("IM_CLIENT_SECRET must be set"),
    );

    let state = AppState {
        db: pool,
        redis,
        http_client,
        im_api_base_url: std::env::var("IM_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.montevideo.gub.uy/api/transportepublico".into()),
        im_auth,
    };

    let app = handlers::router()
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    gtfs_sync::spawn_scheduler(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to port");

    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.expect("server error");
}
