use crate::error::AppResult;
use crate::{AppState, gtfs_sync};

use axum::Json;
use axum::extract::State;

#[derive(serde::Serialize)]
pub struct GtfsVersion {
    pub version: String,
}

pub async fn latest_static_version(State(state): State<AppState>) -> AppResult<Json<GtfsVersion>> {
    let (_, version) = gtfs_sync::fetch(&state).await?;
    Ok(Json(GtfsVersion { version }))
}
