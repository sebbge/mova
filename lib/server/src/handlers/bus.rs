use super::*;

use entities::bus::{Line, RegionBus};

#[derive(serde::Deserialize)]
pub struct RegionQuery {
  region: String,
}

///# GET <MOVA>/buses?region=uy-ma
pub async fn list(State(state): State<crate::State>, Query(query): Query<RegionQuery>) -> crate::Res<Vec<RegionBus>> {
  let buses = Line::region(&state.transit, &query.region).await?;
  tracing::info!(region = query.region, buses = buses.len(), "GET 200 /buses");
  Ok(Json(buses))
}
