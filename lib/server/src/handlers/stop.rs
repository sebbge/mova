use super::*;
use crate::transit::Encoding;

use axum::http::HeaderMap;
use axum::response::Response;
use entities::bus::{Bus, Line};
use entities::stop::{Stop, StopDetail, StopHit, StopSchedule};

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
  Scoped { scope, provider, id: stop_id }: Scoped,
) -> crate::Res<Stop> {
  let stop = Stop::load(&state.transit, &scope, &stop_id).await?;

  tracing::info!(provider, stop = stop_id, lines = stop.lines.len(), "GET 200 /stop/:provider/:id");
  Ok(Json(stop))
}

///# GET <MOVA>/stop/buses/:provider/:id
/// Positions only, from the in-memory fleet
pub async fn buses(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: stop_id }: Scoped,
  Query(params): Query<BusesQuery>,
) -> crate::Res<Vec<Bus>> {
  let stop = Stop::load(&state.transit, &scope, &stop_id).await?;
  let mut line_ids: Vec<String> = stop.lines.iter().map(|l| l.line.clone()).collect();
  line_ids.sort();
  line_ids.dedup();

  let lines = Line::fetch(&state.transit, &scope, &line_ids).await?;
  let mut buses = Line::collect_deduped(&line_ids, &lines);
  if let Some(ids) = params.ids.as_deref() {
    let wanted: std::collections::HashSet<&str> = ids.split(',').collect();
    buses.retain(|b| wanted.contains(b.seen.id.as_str()));
  }
  tracing::info!(provider, stop = stop_id, buses = buses.len(), "GET 200 /stop/buses/:provider/:id");
  Ok(Json(buses))
}

#[derive(Debug, serde::Deserialize)]
pub struct BusesQuery {
  /// Buses already on the map
  pub ids: Option<String>,
}

///# GET <MOVA>/stop/schedule/:provider/:id
/// GTFS only
pub async fn schedule(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: stop_id }: Scoped,
) -> crate::Res<StopSchedule> {
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
  Scoped { scope, provider, id: stop_id }: Scoped,
) -> crate::Res<StopDetail> {
  let detail = StopDetail::load(&state.transit, &scope, &stop_id).await?;

  let rt = detail.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
  tracing::info!(provider, stop = stop_id, lines = detail.lines.len(), rt, "GET 200 /stop/arrivals/:provider/:id");

  Ok(Json(detail))
}

///# GET <MOVA>/stops/search?q=tres cruces&near=-34.9,-56.16
pub async fn search(State(state): State<crate::State>, Query(query): Query<SearchQuery>) -> crate::Res<Vec<StopHit>> {
  let (term, near) = (query.term()?, query.near()?);

  let hits = Stop::load_search(&state.transit, term, near).await?;
  tracing::info!(term, hits = hits.len(), "GET 200 /stops/search");
  Ok(Json(hits))
}
