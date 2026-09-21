use crate::core::geo::Point;
use crate::domain::entities::{
  bus::{Bus, BusFreshness},
  stop::StopArrival,
};

use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct StopGhost {
  pub arrival: StopArrival,
  pub line: String,
}

pub struct BusTracker {
  pub buses: RwLock<HashMap<String, Bus>>,
  stops: RwLock<HashMap<String, HashMap<String, StopGhost>>>,
}

impl BusTracker {
  pub fn new() -> Self {
    Self { buses: RwLock::new(HashMap::new()), stops: RwLock::new(HashMap::new()) }
  }

  pub async fn get(&self, bus_id: &str) -> Option<Bus> {
    self.buses.read().await.get(bus_id).cloned()
  }

  pub async fn missing_ids(&self, line: &str, seen_ids: &HashSet<String>) -> Vec<String> {
    self
      .buses
      .read()
      .await
      .iter()
      .filter(|(id, entry)| !seen_ids.contains(id.as_str()) && entry.line == line)
      .map(|(id, _)| id.clone())
      .collect()
  }

  pub async fn mark_seen(&self, bus_id: &str) {
    let mut map = self.buses.write().await;
    let now = chrono::Utc::now().timestamp();
    map
      .entry(bus_id.to_string())
      .and_modify(|e| {
        e.last_seen_epoch = now;
      })
      .or_insert(Bus {
        id: bus_id.to_string(),
        line: String::new(),
        location: Point { lat: 0.0, lng: 0.0 },
        bearing_deg: None,
        freshness: BusFreshness::Live,
        origin: None,
        destination: None,
        company: String::new(),
        headsign: None,
        timestamp: None,
        still: false,
        speed: 0.0,
        past_stop: false,
        access: None,
        emissions: None,
        thermal_comfort: None,
        stale_count: 0,
        last_seen_epoch: now,
      });
  }

  pub async fn missing(&self, bus_id: &str) -> Option<(BusFreshness, i64)> {
    let mut map = self.buses.write().await;
    let entry = map.get_mut(bus_id)?;
    match BusFreshness::from_missing_s(chrono::Utc::now().timestamp() - entry.last_seen_epoch) {
      Some(f) => Some((f, entry.last_seen_epoch)),
      None => {
        map.remove(bus_id);
        None
      }
    }
  }

  pub async fn remove(&self, bus_id: &str) {
    self.buses.write().await.remove(bus_id);
  }

  pub async fn set_bearing(&self, bus_id: &str, bearing: f64) {
    if let Some(entry) = self.buses.write().await.get_mut(bus_id) {
      entry.bearing_deg = Some(bearing);
    }
  }

  pub async fn stop_ghosts(&self, stop_id: &str) -> HashMap<String, StopGhost> {
    self.stops.read().await.get(stop_id).cloned().unwrap_or_default()
  }

  pub async fn set_stop_ghost(&self, stop_id: &str, bus_id: &str, ghost: StopGhost) {
    self.stops.write().await.entry(stop_id.to_string()).or_default().insert(bus_id.to_string(), ghost);
  }

  pub async fn remove_stop_ghost(&self, stop_id: &str, bus_id: &str) {
    let mut map = self.stops.write().await;
    if let Some(inner) = map.get_mut(stop_id) {
      inner.remove(bus_id);
      if inner.is_empty() {
        map.remove(stop_id);
      }
    }
  }
}
