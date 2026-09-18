use super::*;

use entities::nearby_stop::{NearbyStop, NearbyStopQueryParams};

///# GET <BONDI>/stops/nearby?lat,lng,radius
pub async fn nearby(
  State(state): State<crate::State>,
  Query(params): Query<NearbyStopQueryParams>,
) -> crate::Res<Vec<NearbyStop>> {
  if let Some(cached) = NearbyStop::cache_get(&state.cache, &params).await {
    tracing::info!(count = cached.len(), "GET 200 /stops/nearby (cache hit)");
    return Ok(Json(cached));
  }

  let stops = NearbyStop::get(&state.db, &params).await?;
  NearbyStop::cache_set(&state.cache, &params, &stops).await;

  tracing::info!(count = stops.len(), "GET 200 /stops/nearby");
  Ok(Json(stops))
}
