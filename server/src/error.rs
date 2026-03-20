use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    ExternalApi(String),
    Timeout,
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::Database(e) => {
                tracing::error!("database error: {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    serde_json::json!({ "error": "internal server error" }),
                )
            }
            Self::ExternalApi(msg) => {
                tracing::error!("external API error: {msg}");
                (
                    StatusCode::BAD_GATEWAY,
                    serde_json::json!({ "error": "upstream service unavailable" }),
                )
            }
            Self::Timeout => (
                StatusCode::SERVICE_UNAVAILABLE,
                serde_json::json!({
                    "error": "realtime data unavailable",
                    "fallback": "schedule"
                }),
            ),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, serde_json::json!({ "error": msg })),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, serde_json::json!({ "error": msg })),
        };

        (status, Json(body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else {
            Self::ExternalApi(e.to_string())
        }
    }
}
