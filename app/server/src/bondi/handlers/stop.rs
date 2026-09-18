use super::*;

use entities::{
  bus::Line,
  stop::{StopDetail, StopLine},
};

///# GET <BONDI>/stops/:id
pub async fn stop_detail(State(state): State<crate::State>, Path(stop_id): Path<String>) -> crate::Res<StopDetail> {
  use crate::im_api::UpcomingLinesData;

  if let Some(cached) = StopDetail::cache_get(&state.cache, &stop_id).await {
    let rt = cached.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
    tracing::info!(stop = %stop_id, lines = cached.lines.len(), rt = rt, "GET 200 /stops/:id (cache hit)");
    return Ok(Json(cached));
  }

  // gtfs departures
  let static_lines = StopLine::get(&state.db, &stop_id).await?;
  let line_ids: Vec<String> = static_lines.keys().cloned().collect();

  // im upcoming + line buses
  let (upcoming_lines, live_lines) = {
    let (u, l) = tokio::join!(state.im_api.upcoming_lines(&stop_id, &line_ids), Line::fetch(&state, &line_ids));
    (u?, l.unwrap_or_default())
  };

  // merge
  let detail = StopDetail::build(&state, &stop_id, static_lines, upcoming_lines, live_lines).await;

  StopDetail::cache_set(&state.cache, &stop_id, &detail).await;

  let rt = detail.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
  tracing::info!(stop = %stop_id, lines = detail.lines.len(), rt = rt, "GET 200 /stops/:id");
  println!();

  Ok(Json(detail))
}
