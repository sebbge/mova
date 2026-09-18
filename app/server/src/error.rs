use axum::response::{IntoResponse, Response};
use axum::{Json, http::StatusCode};

#[derive(Debug)]
pub enum Error {
  Database(sqlx::Error),
  ExternalApi(String),
  Timeout,
  RateLimited,
  NotFound(String),
  BadRequest(String),
}
pub type Result<T> = std::result::Result<T, self::Error>;

impl IntoResponse for Error {
  fn into_response(self) -> Response {
    let (status, body) = match self {
      Self::Database(e) => {
        tracing::error!("database error: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, serde_json::json!({ "error": "internal server error" }))
      }
      Self::ExternalApi(msg) => {
        tracing::error!("external API error: {msg}");
        (StatusCode::BAD_GATEWAY, serde_json::json!({ "error": "service unavailable" }))
      }
      Self::Timeout => (StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({"error": "realtime data unavailable",})),
      Self::RateLimited => {
        tracing::warn!("[IMAPI] rate limited (429)");
        (StatusCode::SERVICE_UNAVAILABLE, serde_json::json!({"error": "realtime data unavailable",}))
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

impl From<reqwest::Error> for Error {
  fn from(e: reqwest::Error) -> Self {
    if e.is_timeout() {
      let url = e.url().map(|u| u.as_str()).unwrap_or("unknown");
      tracing::warn!(url = %url, "[IMAPI] request timed out");
      Self::Timeout
    } else {
      let url = e.url().map(|u| u.as_str()).unwrap_or("unknown");
      tracing::error!(url = %url, error = %e, "[IMAPI] request failed");
      Self::ExternalApi(format!("{} (url: {})", e, url))
    }
  }
}
