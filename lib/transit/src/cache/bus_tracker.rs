use crate::entities::{
  bus::{Bus, BusFreshness},
  stop::StopArrival,
};
use crate::sdk::vehicles::BusEntry;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::Instant;

#[derive(Clone)]
pub struct StopGhost {
  pub arrival: StopArrival,
  pub line: String,
}

/// Buses keyed by provider, stops by region
pub fn bus_key(provider: &str, bus_id: &str) -> String {
  format!("{provider}:{bus_id}")
}

pub fn split_bus_key(key: &str) -> Option<(&str, &str)> {
  key.split_once(':')
}

fn stop_key(region: &str, stop_id: &str) -> String {
  format!("{region}:{stop_id}")
}

pub struct Tracked {
  pub bus: Option<Bus>,
  pub stale_count: u8,
  last_seen_epoch: i64,
}

impl Tracked {
  fn seen(bus: Option<Bus>, stale_count: u8) -> Self {
    Self { bus, stale_count, last_seen_epoch: chrono::Utc::now().timestamp() }
  }
}

pub struct FleetView {
  pub at: Instant,
  pub by_line: HashMap<String, Vec<Bus>>,
}

pub struct BusTracker {
  buses: RwLock<HashMap<String, Tracked>>,
  stops: RwLock<HashMap<String, HashMap<String, StopGhost>>>,
  views: RwLock<HashMap<String, Arc<FleetView>>>,
  demand: Mutex<HashMap<String, Instant>>,
  demanded: Notify,
  published: Notify,
}

impl BusTracker {
  pub fn new() -> Self {
    Self {
      buses: RwLock::default(),
      stops: RwLock::default(),
      views: RwLock::default(),
      demand: Mutex::default(),
      demanded: Notify::new(),
      published: Notify::new(),
    }
  }

  pub async fn get(&self, provider: &str, bus_id: &str) -> Option<Bus> {
    self.buses.read().await.get(&bus_key(provider, bus_id)).and_then(|t| t.bus.clone())
  }

  /// Raw ids
  pub async fn missing_ids(&self, provider: &str, line: Option<&str>, seen_ids: &HashSet<String>) -> Vec<String> {
    let prefix = format!("{provider}:");
    self
      .buses
      .read()
      .await
      .iter()
      .filter_map(|(key, entry)| key.strip_prefix(&prefix).map(|raw| (raw, entry)))
      .filter(|(raw, entry)| {
        !seen_ids.contains(*raw) && line.is_none_or(|line| entry.bus.as_ref().is_some_and(|bus| bus.seen.line == line))
      })
      .map(|(raw, _)| raw.to_string())
      .collect()
  }

  pub async fn observe(&self, provider: &str, obs: BusEntry) -> Bus {
    let key = bus_key(provider, &obs.id);
    let mut map = self.buses.write().await;
    let (bus, stale_count) = Bus::from_observation(obs, map.get(&key));
    map.insert(key, Tracked::seen(Some(bus.clone()), stale_count));
    bus
  }

  pub async fn mark_seen(&self, provider: &str, bus_id: &str) {
    let mut map = self.buses.write().await;
    let now = chrono::Utc::now().timestamp();
    map
      .entry(bus_key(provider, bus_id))
      .and_modify(|e| {
        e.last_seen_epoch = now;
      })
      .or_insert_with(|| Tracked::seen(None, 0));
  }

  pub async fn missing(&self, provider: &str, bus_id: &str) -> Option<(BusFreshness, i64)> {
    let key = bus_key(provider, bus_id);
    let mut map = self.buses.write().await;
    let entry = map.get_mut(&key)?;
    match BusFreshness::from_missing_s(chrono::Utc::now().timestamp() - entry.last_seen_epoch) {
      Some(f) => Some((f, entry.last_seen_epoch)),
      None => {
        map.remove(&key);
        None
      }
    }
  }

  pub async fn remove(&self, provider: &str, bus_id: &str) {
    self.buses.write().await.remove(&bus_key(provider, bus_id));
  }

  pub async fn forget(&self, provider: &str) {
    let prefix = format!("{provider}:");
    self.buses.write().await.retain(|key, _| !key.starts_with(&prefix));
  }

  pub async fn set_bearing(&self, provider: &str, bus_id: &str, bearing: f64) {
    if let Some(bus) = self.buses.write().await.get_mut(&bus_key(provider, bus_id)).and_then(|t| t.bus.as_mut()) {
      bus.bearing_deg = Some(bearing);
    }
  }

  /// Keyed by `bus_key`
  pub async fn stop_ghosts(&self, region: &str, stop_id: &str) -> HashMap<String, StopGhost> {
    self.stops.read().await.get(&stop_key(region, stop_id)).cloned().unwrap_or_default()
  }

  pub async fn set_stop_ghost(&self, region: &str, stop_id: &str, bus: &str, ghost: StopGhost) {
    self.stops.write().await.entry(stop_key(region, stop_id)).or_default().insert(bus.to_string(), ghost);
  }

  pub async fn remove_stop_ghost(&self, region: &str, stop_id: &str, bus: &str) {
    let mut map = self.stops.write().await;
    let key = stop_key(region, stop_id);
    if let Some(inner) = map.get_mut(&key) {
      inner.remove(bus);
      if inner.is_empty() {
        map.remove(&key);
      }
    }
  }

  //# fleet
  pub fn touch(&self, provider: &str) {
    if let Ok(mut demand) = self.demand.lock() {
      demand.insert(provider.to_string(), Instant::now());
    }
    self.demanded.notify_waiters();
  }

  pub fn demanded(&self, provider: &str, window: Duration) -> bool {
    let Ok(demand) = self.demand.lock() else {
      return true;
    };
    demand.get(provider).is_some_and(|at| at.elapsed() < window)
  }

  pub async fn await_demand(&self, provider: &str, window: Duration) {
    loop {
      let notified = self.demanded.notified();
      tokio::pin!(notified);
      notified.as_mut().enable();
      if self.demanded(provider, window) {
        return;
      }
      notified.await;
    }
  }

  pub async fn view(&self, provider: &str, max_age: Duration) -> Option<Arc<FleetView>> {
    self.views.read().await.get(provider).filter(|view| view.at.elapsed() < max_age).cloned()
  }

  pub async fn wait_view(&self, provider: &str, max_age: Duration, timeout: Duration) -> Option<Arc<FleetView>> {
    let deadline = Instant::now() + timeout;
    loop {
      let notified = self.published.notified();
      tokio::pin!(notified);
      notified.as_mut().enable();
      if let Some(view) = self.view(provider, max_age).await {
        return Some(view);
      }
      if tokio::time::timeout_at(deadline, notified).await.is_err() {
        return None;
      }
    }
  }

  pub async fn publish(&self, provider: &str, by_line: HashMap<String, Vec<Bus>>) {
    let view = Arc::new(FleetView { at: Instant::now(), by_line });
    self.views.write().await.insert(provider.to_string(), view);
    self.published.notify_waiters();
  }
}
