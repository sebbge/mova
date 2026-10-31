use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Feed version as ETag
pub async fn gtfs_cache(State(state): State<crate::State>, req: Request, next: Next) -> Response {
  let etag = format!("W/\"{}\"", state.transit.versions_token().await);
  let max_age = format!("public, max-age={}, stale-while-revalidate=86400", state.transit.until_sync());
  cached(req, next, &etag, &max_age).await
}

/// Feed version + date as ETag
pub async fn gtfs_daily_cache(State(state): State<crate::State>, req: Request, next: Next) -> Response {
  let today = chrono::Local::now().format("%Y-%m-%d").to_string();
  let etag = format!("W/\"{}-{}\"", state.transit.versions_token().await, today);
  let max_age = format!("public, max-age={}", state.transit.until_midnight());
  cached(req, next, &etag, &max_age).await
}

async fn cached(req: Request, next: Next, etag: &str, cache_control: &str) -> Response {
  if req.headers().get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag) {
    return StatusCode::NOT_MODIFIED.into_response();
  }

  let mut res = next.run(req).await;

  if res.status().is_success() {
    let headers = res.headers_mut();
    if let Ok(value) = HeaderValue::from_str(etag) {
      headers.insert(header::ETAG, value);
    }
    if let Ok(value) = HeaderValue::from_str(cache_control) {
      headers.insert(header::CACHE_CONTROL, value);
    }
  }

  res
}
