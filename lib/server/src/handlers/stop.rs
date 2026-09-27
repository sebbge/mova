use super::*;
use crate::transit::Encoding;

use axum::http::HeaderMap;
use axum::response::Response;
use entities::stop::{Stop, StopDetail, StopSchedule};

#[derive(serde::Deserialize)]
pub struct AreaQuery {
  area: String,
}

///# GET <MOVA>/stops?area=uy-mo
/// Precompressed by the sync
pub async fn list(
  State(state): State<crate::State>,
  Query(query): Query<AreaQuery>,
  headers: HeaderMap,
) -> crate::Result<Response> {
  use axum::http::{HeaderValue, header};

  let area = query.area;
  let enc = Encoding::best(headers.get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok()));

  let body = Stop::load_list(&state.transit, &area, enc).await?;

  tracing::info!(area, bytes = body.len(), encoding = ?enc, "GET 200 /stops");

  let mut res = Response::new(axum::body::Body::from(body));
  let h = res.headers_mut();
  h.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
  // caches must not mix encodings
  h.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
  if let Some(name) = enc.header() {
    h.insert(header::CONTENT_ENCODING, HeaderValue::from_static(name));
  }
  Ok(res)
}

///# GET <MOVA>/stop/:provider/:id
/// GTFS only, no IM quota
pub async fn stop(
  State(state): State<crate::State>,
  Path((provider, stop_id)): Path<(String, String)>,
) -> crate::Res<Stop> {
  let scope = state.transit.resolve(&provider)?;
  let stop = Stop::load(&state.transit, &scope, &stop_id).await?;

  tracing::info!(provider, stop = stop_id, lines = stop.lines.len(), "GET 200 /stop/:provider/:id");
  Ok(Json(stop))
}

///# GET <MOVA>/stop/schedule/:provider/:id
/// GTFS only
pub async fn schedule(
  State(state): State<crate::State>,
  Path((provider, stop_id)): Path<(String, String)>,
) -> crate::Res<StopSchedule> {
  let scope = state.transit.resolve(&provider)?;
  let schedule = StopSchedule::load(&state.transit, &scope, &stop_id).await?;

  let departures: usize = schedule.lines.iter().map(|l| l.departures.len()).sum();
  tracing::info!(
    provider,
    stop = stop_id,
    lines = schedule.lines.len(),
    departures,
    "GET 200 /stop/schedule/:provider/:id"
  );
  Ok(Json(schedule))
}

///# GET <MOVA>/stop/arrivals/:provider/:id
pub async fn arrivals(
  State(state): State<crate::State>,
  Path((provider, stop_id)): Path<(String, String)>,
) -> crate::Res<StopDetail> {
  let scope = state.transit.resolve(&provider)?;
  let detail = StopDetail::load(&state.transit, &scope, &stop_id).await?;

  let rt = detail.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
  tracing::info!(provider, stop = stop_id, lines = detail.lines.len(), rt, "GET 200 /stop/arrivals/:provider/:id");

  Ok(Json(detail))
}
