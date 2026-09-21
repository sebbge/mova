use super::*;

#[derive(serde::Serialize)]
pub struct HealthResponse {
  pub status: &'static str,
  pub realtime: bool,
}

///# GET <BONDI>/health
pub async fn health(State(state): State<crate::State>) -> (StatusCode, Json<HealthResponse>) {
  let realtime = state.provider.probe().await;

  tracing::info!(realtime, "GET 200 /health");
  let response = HealthResponse { status: "ok", realtime };

  (StatusCode::OK, Json(response))
}
