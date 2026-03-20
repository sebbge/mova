use crate::AppState;

use axum::{Router, routing::get};

pub mod arrivals;
pub mod gtfs;
pub mod lines;
pub mod routes;
pub mod stops;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/stops/nearby", get(stops::nearby))
        .route(
            "/api/v1/realtime/buses/{route_id}",
            get(lines::bus_positions),
        )
        .route("/api/v1/routes/search", get(routes::search))
        .route("/api/v1/routes/{route_id}/shape", get(routes::shape))
        .route(
            "/api/v1/stops/{stop_id}/arrivals",
            get(arrivals::stop_arrivals),
        )
        .route(
            "/api/v1/gtfs/static/version",
            get(gtfs::latest_static_version),
        )
}
