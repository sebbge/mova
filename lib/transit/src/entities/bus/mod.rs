use super::*;
use crate::scope::Scope;

mod repository;

//# Bus def
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
  #[serde(flatten)]
  pub seen: BusEntry,

  pub headsign: Option<String>,
  pub bearing_deg: Option<f64>,
  pub freshness: BusFreshness,
  pub still: bool,
  pub past_stop: bool,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusFreshness {
  #[default]
  Live,
  LowPrecision,
  Imprecise,
}
impl BusFreshness {
  const LOW_PRECISION_S: i64 = 30;
  const GHOST_MAX_S: i64 = 70;

  pub fn from_missing_s(missing_s: i64) -> Option<Self> {
    if missing_s <= Self::LOW_PRECISION_S {
      Some(Self::LowPrecision)
    } else if missing_s <= Self::GHOST_MAX_S {
      Some(Self::Imprecise)
    } else {
      None
    }
  }
}

//# Line def
use crate::cache::Tracked;
use crate::sdk::vehicles::{BusEntry, StopRef, Vehicles};

use std::time::Duration;

pub type Headsigns = (Vec<String>, Vec<String>);

/// Columns for `unnest`
struct Positions {
  ids: Vec<String>,
  lats: Vec<f64>,
  lngs: Vec<f64>,
  dests: Vec<Option<String>>,
}

impl Positions {
  fn without_next_stop(buses: &[Bus]) -> Self {
    let missing = buses.iter().filter(|b| b.seen.next_stop.is_none());
    let mut p = Positions { ids: vec![], lats: vec![], lngs: vec![], dests: vec![] };
    for b in missing {
      p.ids.push(b.seen.id.clone());
      p.lats.push(b.seen.location.lat);
      p.lngs.push(b.seen.location.lng);
      p.dests.push(b.seen.destination.clone());
    }
    p
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Line {
  pub line_id: String,
  pub company: String,
  pub destination: Option<String>,
  pub buses: Vec<Bus>,
}

//# Bus impl
impl Bus {
  const STALE_DIST: f64 = 20.0;
  const BEARING_DIST: f64 = 3.0;
  const STALE_THRESHOLD: u8 = 3;

  pub fn live(seen: BusEntry) -> Self {
    Self { seen, headsign: None, bearing_deg: None, freshness: BusFreshness::Live, still: false, past_stop: false }
  }

  pub fn from_observation(mut obs: BusEntry, prev: Option<&Tracked>) -> (Self, u8) {
    let last = prev.and_then(|t| t.bus.as_ref().map(|bus| (bus, t.stale_count)));
    let repeated = last.filter(|(bus, _)| bus.seen.timestamp.is_some() && bus.seen.timestamp == obs.timestamp);

    let (bearing_deg, still, stale_count) = if let Some((bus, stale_count)) = repeated {
      (bus.bearing_deg, bus.still, stale_count)
    } else if let Some((bus, stale_count)) = last {
      let dist = bus.seen.location.distance_to(&obs.location);

      let bearing_deg =
        if dist >= Self::BEARING_DIST { Some(bus.seen.location.bearing_to(&obs.location)) } else { bus.bearing_deg };

      let (still, stale_count) = if dist < Self::STALE_DIST {
        let count = (stale_count + 1).min(Self::STALE_THRESHOLD);
        (count >= Self::STALE_THRESHOLD, count)
      } else {
        (false, 0)
      };

      (bearing_deg, still, stale_count)
    } else {
      (None, false, 0)
    };

    if still {
      obs.speed = Some(0.0);
    }
    (Self { bearing_deg, still, ..Self::live(obs) }, stale_count)
  }
}

//# Region
/// Bus with its provider
#[derive(Debug, Serialize)]
pub struct RegionBus {
  pub provider: String,
  #[serde(flatten)]
  pub bus: Bus,
}

//# Line impl
pub type Lines = HashMap<String, Line>;
impl Line {
  pub const FLEET_STALE: Duration = Duration::from_secs(60);
  const FLEET_WAIT: Duration = Duration::from_secs(15);

  pub async fn fetch(state: &crate::Transit, scope: &Scope, line_ids: &[String]) -> crate::Result<Lines> {
    use crate::cache::BusTracking;

    let tracker = state.cache.tracker();
    if let Some(Vehicles::Fleet(_)) = scope.vehicles() {
      tracker.touch(scope.provider);
    }

    let mut lines: Lines = HashMap::new();
    let mut uncached: Vec<String> = Vec::new();

    for line_id in line_ids {
      if let Some(cached) = Line::cache_get(&state.cache, scope.region.id(), scope.provider, line_id).await {
        lines.insert(line_id.clone(), cached);
      } else {
        uncached.push(line_id.clone());
      }
    }
    if uncached.is_empty() {
      return Ok(lines);
    }

    match scope.vehicles() {
      None => {}
      Some(Vehicles::Lines(vehicles)) => {
        let mut by_line = vehicles.buses(&uncached).await?;
        for line_id in &uncached {
          let observations = by_line.remove(line_id).unwrap_or_default();
          let line = Line::build(state, scope, line_id, observations).await;
          lines.insert(line_id.clone(), line);
        }
      }
      Some(Vehicles::Fleet(_)) => {
        let view =
          tracker.wait_view(scope.provider, Self::FLEET_STALE, Self::FLEET_WAIT).await.ok_or(crate::Error::Timeout)?;
        for line_id in &uncached {
          let buses = view.by_line.get(line_id).cloned().unwrap_or_default();
          let line = Line::enrich(state, scope, line_id, buses).await;
          lines.insert(line_id.clone(), line);
        }
      }
    }

    Ok(lines)
  }

  /// Fleet buses of an authority, for small regions
  pub async fn region(state: &crate::Transit, region: &str) -> crate::Result<Vec<RegionBus>> {
    let scopes: Vec<Scope> = state.scopes().filter(|s| s.region.id() == region).collect();
    if scopes.is_empty() {
      return Err(crate::Error::NotFound(format!("unknown region: {region}")));
    }

    let mut views = tokio::task::JoinSet::new();
    for scope in scopes.into_iter().filter(|s| matches!(s.vehicles(), Some(Vehicles::Fleet(_)))) {
      let state = state.clone();
      views.spawn(async move {
        use crate::cache::BusTracking;
        let tracker = state.cache.tracker();
        tracker.touch(scope.provider);
        let view = tracker.wait_view(scope.provider, Self::FLEET_STALE, Self::FLEET_WAIT).await;
        (scope.provider, view)
      });
    }

    let mut buses = Vec::new();
    while let Some(joined) = views.join_next().await {
      match joined {
        Ok((provider, Some(view))) => buses.extend(
          view.by_line.values().flatten().map(|bus| RegionBus { provider: provider.to_string(), bus: bus.clone() }),
        ),
        Ok((provider, None)) => tracing::warn!(provider, "[fleet] no view in time, left out of region"),
        Err(e) => tracing::warn!(error = ?e, "[fleet] region view task failed"),
      }
    }
    Ok(buses)
  }

  pub async fn observe_fleet(state: &crate::Transit, scope: &Scope, fleet: Vec<BusEntry>) {
    use crate::cache::BusTracking;

    let tracker = state.cache.tracker();
    let provider = scope.provider;
    if tracker.view(provider, Self::FLEET_STALE).await.is_none() {
      tracker.forget(provider).await;
    }

    let mut by_line: HashMap<String, Vec<Bus>> = HashMap::new();
    let mut seen_ids = HashSet::new();
    for obs in fleet {
      if !seen_ids.insert(obs.id.clone()) {
        continue;
      }
      let bus = tracker.observe(provider, obs).await;
      by_line.entry(bus.seen.line.clone()).or_default().push(bus);
    }

    let ghosts = Self::ghosts(tracker, provider, None, &seen_ids).await;
    tracing::debug!(provider, live = seen_ids.len(), ghosts = ghosts.len(), "[fleet] observed");
    for ghost in ghosts {
      by_line.entry(ghost.seen.line.clone()).or_default().push(ghost);
    }

    Self::remember_companies(&state.db, scope, by_line.values().flatten()).await;
    tracker.publish(provider, by_line).await;
  }

  async fn build(state: &crate::Transit, scope: &Scope, line: &str, observations: Vec<BusEntry>) -> Self {
    use crate::cache::BusTracking;

    let tracker = state.cache.tracker();
    let provider = scope.provider;

    let mut buses = Vec::new();
    let mut seen_ids = HashSet::new();
    for obs in observations {
      seen_ids.insert(obs.id.clone());
      buses.push(tracker.observe(provider, obs).await);
    }
    Self::remember_companies(&state.db, scope, buses.iter()).await;
    buses.extend(Self::ghosts(tracker, provider, Some(line), &seen_ids).await);

    Self::enrich(state, scope, line, buses).await
  }

  async fn ghosts(
    tracker: &crate::cache::BusTracker,
    provider: &str,
    line: Option<&str>,
    seen_ids: &HashSet<String>,
  ) -> Vec<Bus> {
    let mut ghosts = Vec::new();
    for id in tracker.missing_ids(provider, line, seen_ids).await {
      let Some((freshness, _)) = tracker.missing(provider, &id).await else {
        continue;
      };
      let Some(mut bus) = tracker.get(provider, &id).await else {
        continue;
      };
      if bus.seen.line.is_empty() || bus.seen.timestamp.is_none() || bus.seen.company.is_empty() {
        continue;
      }
      bus.freshness = freshness;
      bus.still = true;
      ghosts.push(bus);
    }
    ghosts
  }

  async fn enrich(state: &crate::Transit, scope: &Scope, line: &str, mut buses: Vec<Bus>) -> Self {
    let headsigns = Self::load_headsigns(state, scope, line).await;
    for bus in &mut buses {
      bus.headsign = Self::compute_headsign(&bus.seen.origin, &bus.seen.destination, &headsigns);
    }

    let missing_stops = Positions::without_next_stop(&buses);
    let (_, next_stops) = tokio::join!(
      Self::fill_missing_bearings(state, scope, line, &mut buses),
      Self::locate_next_stops(state, scope, line, missing_stops)
    );
    for bus in &mut buses {
      if let Some(stop) = next_stops.get(&bus.seen.id) {
        bus.seen.next_stop = Some(stop.clone());
      }
    }

    Self::cache_set(&state.cache, scope.region.id(), scope.provider, line, &buses).await;

    let destination = buses.first().map(|b| b.seen.destination.clone()).unwrap_or_default();
    let company = buses.first().map(|b| b.seen.company.clone()).unwrap_or_default();
    Self { line_id: line.to_string(), company, destination, buses }
  }

  async fn fill_missing_bearings(state: &crate::Transit, scope: &Scope, line: &str, positions: &mut [Bus]) {
    let missing: Vec<usize> =
      positions.iter().enumerate().filter(|(_, p)| p.bearing_deg.is_none()).map(|(i, _)| i).collect();

    if missing.is_empty() {
      return;
    }

    let mut p_ids: Vec<String> = Vec::with_capacity(missing.len());
    let mut p_lats: Vec<f64> = Vec::with_capacity(missing.len());
    let mut p_lngs: Vec<f64> = Vec::with_capacity(missing.len());
    let mut p_dests: Vec<Option<String>> = Vec::with_capacity(missing.len());

    for &i in &missing {
      let p = &positions[i];
      p_ids.push(p.seen.id.clone());
      p_lats.push(p.seen.location.lat);
      p_lngs.push(p.seen.location.lng);
      p_dests.push(p.seen.destination.clone());
    }

    let rows = Self::query_bearings(&state.db, scope, line, &p_ids, &p_lats, &p_lngs, &p_dests).await;

    let bearing_map: HashMap<&str, f64> =
      rows.iter().filter_map(|r| r.bearing_deg.map(|b| (r.bus_id.as_str(), b))).collect();

    use crate::cache::BusTracking;
    for &i in &missing {
      let p = &mut positions[i];
      if let Some(&deg) = bearing_map.get(p.seen.id.as_str()) {
        p.bearing_deg = Some(deg);
        state.cache.tracker().set_bearing(scope.provider, &p.seen.id, deg).await;
      }
    }
  }

  /// Next stop ahead on the nearest shape, for sources without one
  async fn locate_next_stops(
    state: &crate::Transit,
    scope: &Scope,
    line: &str,
    p: Positions,
  ) -> HashMap<String, StopRef> {
    if p.ids.is_empty() {
      return HashMap::new();
    }

    let rows = Self::query_next_stops(&state.db, scope, line, &p).await;

    rows.into_iter().map(|r| (r.bus_id, StopRef { id: r.stop_id, name: r.stop_name })).collect()
  }

  async fn load_headsigns(state: &crate::Transit, scope: &Scope, route_short_name: &str) -> Headsigns {
    if let Some(cached) =
      Self::get_headsigns_cache(&state.cache, scope.region.id(), scope.provider, route_short_name).await
    {
      return cached;
    }

    let rows = Self::query_headsigns(&state.db, scope, route_short_name).await;

    let mut hs: Headsigns = (Vec::new(), Vec::new());
    for r in rows {
      if let Some(h) = r.trip_headsign {
        match r.direction_id {
          0 => hs.0.push(h),
          1 => hs.1.push(h),
          _ => {}
        }
      }
    }

    Self::set_headsigns_cache(&state.cache, scope.region.id(), scope.provider, route_short_name, &hs).await;
    hs
  }

  fn compute_headsign(origin: &Option<String>, destination: &Option<String>, headsigns: &Headsigns) -> Option<String> {
    let norm = |s: &str| s.to_uppercase().trim().to_string();
    let matches = |list: &[String], val: &str| list.iter().any(|h| norm(h) == val);

    let (hs0, hs1) = headsigns;

    if let Some(dest) = destination {
      let d = norm(dest);
      if matches(hs0, &d) {
        return Some("Ida".into());
      }
      if matches(hs1, &d) {
        return Some("Vuelta".into());
      }
    }

    if let Some(orig) = origin {
      let o = norm(orig);
      if matches(hs1, &o) {
        return Some("Ida".into());
      }
      if matches(hs0, &o) {
        return Some("Vuelta".into());
      }
    }

    None
  }

  pub fn collect_deduped(line_ids: &[String], bus_positions: &Lines) -> Vec<Bus> {
    let mut best: HashMap<String, usize> = HashMap::new();
    let mut deduped: Vec<Bus> = Vec::new();
    for line_id in line_ids {
      let Some(line) = bus_positions.get(line_id) else {
        continue;
      };
      for bus in &line.buses {
        if let Some(&idx) = best.get(&bus.seen.id) {
          if bus.freshness == BusFreshness::Live && deduped[idx].freshness != BusFreshness::Live {
            deduped[idx] = bus.clone();
          }
        } else {
          best.insert(bus.seen.id.clone(), deduped.len());
          deduped.push(bus.clone());
        }
      }
    }
    deduped
  }
}
