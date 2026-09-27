use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Feed version as ETag
pub async fn gtfs_cache(State(state): State<crate::State>, req: Request, next: Next) -> Response {
  let etag = format!("W/\"{}\"", state.transit.versions_token().await);

  if req.headers().get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
    return StatusCode::NOT_MODIFIED.into_response();
  }

  let mut res = next.run(req).await;

  if res.status().is_success() {
    let headers = res.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&etag) {
      headers.insert(header::ETAG, value);
    }
    let cache_control = format!("public, max-age={}, stale-while-revalidate=86400", state.transit.until_sync());
    if let Ok(value) = HeaderValue::from_str(&cache_control) {
      headers.insert(header::CACHE_CONTROL, value);
    }
  }

  res
}

/// Feed version + date as ETag
pub async fn gtfs_daily_cache(State(state): State<crate::State>, req: Request, next: Next) -> Response {
  let today = chrono::Local::now().format("%Y-%m-%d").to_string();
  let etag = format!("W/\"{}-{}\"", state.transit.versions_token().await, today);

  if req.headers().get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
    return StatusCode::NOT_MODIFIED.into_response();
  }

  let mut res = next.run(req).await;

  if res.status().is_success() {
    let headers = res.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&etag) {
      headers.insert(header::ETAG, value);
    }
    let cache_control = format!("public, max-age={}", crate::transit::secs_to_midnight());
    if let Ok(value) = HeaderValue::from_str(&cache_control) {
      headers.insert(header::CACHE_CONTROL, value);
    }
  }

  res
}
