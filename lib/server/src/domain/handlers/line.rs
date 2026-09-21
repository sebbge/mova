use super::*;

use entities::bus::{Bus, BusFreshness, Line};

///# GET <BONDI>/lines/:id
pub async fn buses(State(state): State<crate::State>, Path(line_id): Path<String>) -> crate::Res<Vec<Bus>> {
  let mut lines = Line::fetch(&state, std::slice::from_ref(&line_id)).await?;
  let buses = lines.remove(&line_id).map(|l| l.buses).unwrap_or_default();

  tracing::info!(
    line = %line_id,
    live = buses.iter().filter(|b| b.freshness == BusFreshness::Live).count(),
    ghosts = buses.iter().filter(|b| b.freshness != BusFreshness::Live).count(),
    "GET 200 /lines/:id"
  );

  Ok(Json(buses))
}
