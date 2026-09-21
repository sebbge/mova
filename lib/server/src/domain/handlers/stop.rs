use super::*;

use entities::{
  bus::Line,
  stop::{Stop, StopDetail, StopLine, StopSchedule, StopSummary},
};

///# GET <BONDI>/stops
pub async fn list(State(state): State<crate::State>) -> crate::Res<Vec<StopSummary>> {
  if let Some(cached) = Stop::cache_get_list(&state.cache).await {
    return Ok(Json(cached));
  }

  let stops = Stop::list(&state.db).await?;
  Stop::cache_set_list(&state.cache, &stops).await;

  tracing::info!(count = stops.len(), "GET 200 /stops");
  Ok(Json(stops))
}

///# GET <BONDI>/stops/:id
/// GTFS only, no IM quota
pub async fn stop(State(state): State<crate::State>, Path(stop_id): Path<String>) -> crate::Res<Stop> {
  if let Some(cached) = Stop::cache_get(&state.cache, &stop_id).await {
    return Ok(Json(cached));
  }

  let stop = Stop::get(&state.db, &stop_id).await?;
  Stop::cache_set(&state.cache, &stop_id, &stop).await;

  tracing::info!(stop = %stop_id, lines = stop.lines.len(), "GET 200 /stops/:id");
  Ok(Json(stop))
}

///# GET <BONDI>/stops/:id/schedule
/// GTFS only
pub async fn schedule(State(state): State<crate::State>, Path(stop_id): Path<String>) -> crate::Res<StopSchedule> {
  let today = chrono::Local::now().format("%Y-%m-%d").to_string();

  if let Some(cached) = StopSchedule::cache_get(&state.cache, &stop_id, &today).await {
    return Ok(Json(cached));
  }

  let schedule = StopSchedule::get(&state.db, &stop_id).await?;
  StopSchedule::cache_set(&state.cache, &stop_id, &schedule).await;

  let departures: usize = schedule.lines.iter().map(|l| l.departures.len()).sum();
  tracing::info!(stop = %stop_id, lines = schedule.lines.len(), departures, "GET 200 /stops/:id/schedule");
  Ok(Json(schedule))
}

///# GET <BONDI>/stops/:id/arrivals
pub async fn arrivals(State(state): State<crate::State>, Path(stop_id): Path<String>) -> crate::Res<StopDetail> {
  if let Some(cached) = StopDetail::cache_get(&state.cache, &stop_id).await {
    let rt = cached.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
    tracing::info!(stop = %stop_id, lines = cached.lines.len(), rt = rt, "GET 200 /stops/:id/arrivals (cache hit)");
    return Ok(Json(cached));
  }

  // gtfs departures
  let static_lines = StopLine::get(&state.db, &stop_id).await?;
  let line_ids: Vec<String> = static_lines.keys().cloned().collect();

  // im upcoming + line buses
  let (upcoming_lines, live_lines) = {
    let (u, l) = tokio::join!(state.provider.upcoming(&stop_id, &line_ids), Line::fetch(&state, &line_ids));
    (u.unwrap_or_default(), l.unwrap_or_default())
  };

  // merge
  let detail = StopDetail::build(&state, &stop_id, static_lines, upcoming_lines, live_lines).await;

  StopDetail::cache_set(&state.cache, &stop_id, &detail).await;

  let rt = detail.lines.iter().flat_map(|l| &l.arrivals).filter(|a| a.live).count();
  tracing::info!(stop = %stop_id, lines = detail.lines.len(), rt = rt, "GET 200 /stops/:id/arrivals");

  Ok(Json(detail))
}
