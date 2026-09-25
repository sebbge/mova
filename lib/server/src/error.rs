use axum::response::{IntoResponse, Response};
use axum::{Json, http::StatusCode};

#[derive(Debug)]
pub enum Error {
  Config(String),
  Database(sqlx::Error),
  External(String),
  Timeout,
  RateLimited,
  NotFound(String),
  BadRequest(String),
}
pub type Result<T> = std::result::Result<T, Error>;

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let (status, body) = match self {
      Self::Config(msg) => {
        tracing::error!(error = %msg, "[api] config error");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::Database(e) => {
        tracing::error!(error = %e, "[api] database error");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::External(msg) => {
        tracing::error!(error = %msg, "[api] external source error");
        (StatusCode::BAD_GATEWAY, serde_json::json!({ "error": "service unavailable" }))
      }
      Self::Timeout | Self::RateLimited => {
        (StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({ "error": "realtime data unavailable" }))
      }
      Self::NotFound(msg) => (StatusCode::NOT_FOUND, serde_json::json!({ "error": msg })),
      Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, serde_json::json!({ "error": msg })),
    };

    (status, Json(body)).into_response()
  }
}

impl From<sqlx::Error> for Error {
  fn from(e: sqlx::Error) -> Self {
    Self::Database(e)
  }
}

impl From<crate::core::Error> for Error {
  fn from(e: crate::core::Error) -> Self {
    use crate::core::Error as Core;
    match e {
      Core::Config(msg) => Self::Config(msg),
      Core::External(msg) => Self::External(msg),
      Core::Timeout => Self::Timeout,
      Core::RateLimited => Self::RateLimited,
    }
  }
}
