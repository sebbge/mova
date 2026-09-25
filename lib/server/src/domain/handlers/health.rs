use super::*;
use crate::core::provider::Vehicles;

use entities::bus::Line;

#[derive(serde::Serialize)]
pub struct HealthResponse {
  pub status: &'static str,
  /// One per provider
  pub realtime: std::collections::BTreeMap<String, bool>,
}

///# GET <BONDI>/health
pub async fn health(State(state): State<crate::State>) -> (StatusCode, Json<HealthResponse>) {
  let mut realtime = std::collections::BTreeMap::new();

  for scope in state.scopes() {
    use crate::cache::BusTracking;
    let up = match scope.vehicles() {
      Some(Vehicles::Lines(vehicles)) => vehicles.probe().await,
      Some(Vehicles::Fleet(fleet)) => {
        state.cache.tracker().view(&scope.provider, Line::FLEET_STALE).await.is_some() || fleet.fleet().await.is_ok()
      }
      None => false,
    };
    realtime.insert(scope.provider.clone(), up);
  }

  tracing::info!(?realtime, "GET 200 /health");
  let response = HealthResponse { status: "ok", realtime };

  (StatusCode::OK, Json(response))
}
