use crate::entities::bus::Line;
use crate::sdk::vehicles::Vehicles;

use std::time::Duration;

const INTERVAL: Duration = Duration::from_secs(10);
const DEMAND: Duration = Duration::from_secs(120);

pub fn spawn_pollers(state: &crate::Transit) {
  for scope in state.scopes() {
    let Some(Vehicles::Fleet(fleet)) = scope.vehicles().cloned() else {
      continue;
    };
    let state = state.clone();

    tokio::spawn(async move {
      let tracker = state.cache.tracker();
      loop {
        if !tracker.demanded(scope.provider, DEMAND) {
          tracing::info!(provider = scope.provider, "[fleet] idle");
          tracker.await_demand(scope.provider, DEMAND).await;
          tracing::info!(provider = scope.provider, "[fleet] polling");
        }

        if let Ok(buses) = fleet.fleet().await {
          Line::observe_fleet(&state, &scope, buses).await;
        }
        tokio::time::sleep(INTERVAL).await;
      }
    });
  }
}
