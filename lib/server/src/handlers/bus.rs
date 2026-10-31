use super::*;

use entities::bus::{Bus, BusFreshness, Line, RegionBus};

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

///# GET <MOVA>/line/buses/:provider/:id
pub async fn line(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: line_id }: Scoped,
) -> crate::Res<Vec<Bus>> {
  let ids = [line_id.clone()];

  let mut lines = Line::fetch(&state.transit, &scope, &ids).await?;
  let buses = lines.remove(&line_id).map(|l| l.buses).unwrap_or_default();

  tracing::info!(
    provider,
    line = line_id,
    live = buses.iter().filter(|b| b.freshness == BusFreshness::Live).count(),
    ghosts = buses.iter().filter(|b| b.freshness != BusFreshness::Live).count(),
    "GET 200 /line/buses/:provider/:id"
  );

  Ok(Json(buses))
}
