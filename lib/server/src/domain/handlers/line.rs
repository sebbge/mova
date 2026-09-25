use super::*;

use entities::bus::{Bus, BusFreshness, Line};

///# GET <BONDI>/line/buses/:provider/:id
pub async fn buses(
  State(state): State<crate::State>,
  Path((provider, line_id)): Path<(String, String)>,
) -> crate::Res<Vec<Bus>> {
  let scope = state.resolve(&provider)?;
  let ids = [line_id.clone()];

  let mut lines = Line::fetch(&state, &scope, &ids).await?;
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
