use crate::planner::Error as Planner;
use crate::transit::Error as Transit;

use axum::response::{IntoResponse, Response};
use axum::{Json, http::StatusCode};

#[derive(Debug)]
pub enum Error {
  Transit(Transit),
  Planner(Planner),
  BadRequest(String),
}
pub type Result<T> = std::result::Result<T, Error>;

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let (status, body) = match self {
      Self::Transit(Transit::Config(msg)) => {
        tracing::error!(error = %msg, "[api] config error");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::Transit(Transit::Database(e)) => {
        tracing::error!(error = %e, "[api] database error");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::Transit(Transit::External(msg)) => {
        tracing::error!(error = %msg, "[api] external source error");
        (StatusCode::BAD_GATEWAY, serde_json::json!({ "error": "service unavailable" }))
      }
      Self::Transit(Transit::Timeout | Transit::RateLimited) => {
        (StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({ "error": "realtime data unavailable" }))
      }
      Self::Transit(Transit::NotFound(msg)) => (StatusCode::NOT_FOUND, serde_json::json!({ "error": msg })),
      Self::Planner(Planner::Transit(e)) => return Self::Transit(e).into_response(),
      Self::Planner(Planner::NotReady) => {
        (StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({ "error": "planner not ready" }))
      }
      Self::Planner(Planner::Internal(msg)) => {
        tracing::error!(error = %msg, "[api] planner error");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, serde_json::json!({ "error": msg })),
    };

    (status, Json(body)).into_response()
  }
}

impl From<Planner> for Error {
  fn from(e: Planner) -> Self {
    Self::Planner(e)
  }
}

impl From<Transit> for Error {
  fn from(e: Transit) -> Self {
    Self::Transit(e)
  }
}
