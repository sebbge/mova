use super::*;

use std::collections::BTreeMap;

#[derive(serde::Serialize)]
pub struct HealthResponse {
  pub status: &'static str,
  pub realtime: BTreeMap<String, bool>,
}

///# GET <MOVA>/health
pub async fn health(State(state): State<crate::State>) -> (StatusCode, Json<HealthResponse>) {
  let realtime = state.transit.realtime().await;

  tracing::info!(?realtime, "GET 200 /health");
  let response = HealthResponse { status: "ok", realtime };

  (StatusCode::OK, Json(response))
}
