use crate::AppState;
use crate::error::AppResult;
use crate::models::{BusPosition, ImBus};

use axum::Json;
use axum::extract::{Path, State};
use redis::AsyncCommands;

pub async fn bus_positions(
    State(state): State<AppState>,
    Path(route_id): Path<String>,
) -> AppResult<Json<Vec<BusPosition>>> {
    let cache_key = format!("realtime:buses:{route_id}");

    let mut redis = state.redis.clone();
    if let Ok(cached) = redis.get::<_, String>(&cache_key).await
        && let Ok(positions) = serde_json::from_str::<Vec<BusPosition>>(&cached)
    {
        return Ok(Json(positions));
    }

    let token = state.im_auth.token().await?;
    let url = format!("{}/buses?lines={route_id}", state.im_api_base_url);

    let buses: Vec<ImBus> = state
        .http_client
        .get(&url)
        .bearer_auth(&token)
        .send()
        .await?
        .json()
        .await?;

    let positions: Vec<BusPosition> = buses
        .into_iter()
        .filter_map(|b| b.into_position())
        .collect();

    if let Ok(json) = serde_json::to_string(&positions) {
        let _: Result<(), _> = redis.set_ex(&cache_key, json, 10u64).await;
    }

    Ok(Json(positions))
}
