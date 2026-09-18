use super::*;

mod repository;

//# Bus def
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bus {
  pub id: String,
  pub line: String,
  pub company: String,
  pub location: Point,
  pub speed: f64,

  pub origin: Option<String>,
  pub destination: Option<String>,
  pub headsign: Option<String>,

  pub timestamp: Option<String>,
  //
  pub access: Option<String>,
  pub emissions: Option<String>,
  pub thermal_comfort: Option<String>,
  //
  pub bearing_deg: Option<f64>,
  pub freshness: BusFreshness,
  pub still: bool,
  pub past_stop: bool,
  //
  pub stale_count: u8,
  pub last_seen_epoch: i64,
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
use crate::im_api::models::{ImLineBus, Point};

pub type Headsigns = (Vec<String>, Vec<String>);

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

  pub fn from_im_line_bus(im_bus: ImLineBus, prev: Option<&Self>) -> Self {
    let (bearing_deg, still, stale_count) = if let Some(prev) = prev {
      let dist = prev.location.get_distance_to(&im_bus.bus.location);

      let bearing_deg = if dist >= Self::BEARING_DIST {
        Some(prev.location.get_bearing_to(&im_bus.bus.location))
      } else {
        prev.bearing_deg
      };

      let (still, stale_count) = if dist < Self::STALE_DIST {
        let count = (prev.stale_count + 1).min(Self::STALE_THRESHOLD);
        (count >= Self::STALE_THRESHOLD, count)
      } else {
        (false, 0)
      };

      (bearing_deg, still, stale_count)
    } else {
      (None, false, 0)
    };

    Self {
      id: im_bus.bus.bus_id.to_string(),
      line: im_bus.bus.line.clone(),
      company: im_bus.bus.company.clone(),
      location: im_bus.bus.location.clone(),
      speed: if still { 0.0 } else { im_bus.speed.unwrap_or(0.0) },
      //
      origin: Some(im_bus.bus.origin.clone()),
      destination: Some(im_bus.bus.destination.clone()),
      headsign: None,

      timestamp: im_bus.timestamp.map(|t| t.to_rfc3339()),
      //
      access: im_bus.bus.access.label(),
      emissions: im_bus.bus.emissions.label(),
      thermal_comfort: im_bus.bus.thermal_confort.label(),
      //
      bearing_deg,
      freshness: BusFreshness::Live,
      still,
      past_stop: false,
      stale_count,
      last_seen_epoch: chrono::Utc::now().timestamp(),
    }
  }

  pub fn get_speed(&self) -> f64 {
    if self.still {
      return 0.0;
    }
    if self.speed >= 5.0 { self.speed } else { 20.0 }
  }
}

//# Line impl
pub type Lines = HashMap<String, Line>;
impl Line {
  pub async fn fetch(state: &crate::State, line_ids: &[String]) -> crate::Result<Lines> {
    use crate::im_api::LinesData;

    let mut lines: Lines = HashMap::new();
    let mut uncached: Vec<String> = Vec::new();

    for line_id in line_ids {
      if let Some(cached) = Line::cache_get(&state.cache, line_id).await {
        lines.insert(line_id.clone(), cached);
      } else {
        uncached.push(line_id.clone());
      }
    }
    if uncached.is_empty() {
      return Ok(lines);
    }

    let mut by_line = state.im_api.lines(&uncached).await?;

    for line_id in &uncached {
      let im_buses = by_line.remove(line_id).unwrap_or_default();
      let line = Line::build(state, line_id, im_buses).await;
      lines.insert(line_id.clone(), line);
    }

    Ok(lines)
  }

  async fn build(state: &crate::State, line: &str, im_buses: Vec<ImLineBus>) -> Self {
    use crate::cache::BusTracking;

    let tracker = state.cache.tracker();
    let headsigns = Self::load_headsigns(state, line).await;

    let mut buses = Vec::new();
    let mut seen_ids = HashSet::new();
    for im_bus in im_buses {
      let id = im_bus.bus.bus_id.to_string();

      let mut tracked_buses = tracker.buses.write().await;
      let prev = tracked_buses.get(&id).cloned();
      let mut bus = Bus::from_im_line_bus(im_bus, prev.as_ref());
      bus.headsign = Self::compute_headsign(&bus.origin, &bus.destination, &headsigns);
      seen_ids.insert(id.clone());
      buses.push(bus.clone());
      tracked_buses.insert(id, bus);
    }

    for id in tracker.missing_ids(line, &seen_ids).await {
      let Some((freshness, _)) = tracker.missing(&id).await else {
        continue;
      };
      let Some(entry) = tracker.get(&id).await else { continue };
      if entry.line.is_empty() || entry.timestamp.is_none() || entry.company.is_empty() {
        continue;
      }
      let mut bus = entry;
      bus.freshness = freshness;
      bus.headsign = Self::compute_headsign(&bus.origin, &bus.destination, &headsigns);
      bus.still = true;
      buses.push(bus);
    }

    Self::fill_missing_bearings(state, line, &mut buses).await;
    Self::cache_set(&state.cache, line, &buses).await;

    let destination = buses.first().map(|b| b.destination.clone()).unwrap_or_default();
    let company = buses.first().map(|b| b.company.clone()).unwrap_or_default();
    Self { line_id: line.to_string(), company, destination, buses }
  }

  async fn fill_missing_bearings(state: &crate::State, line: &str, positions: &mut [Bus]) {
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
      p_ids.push(p.id.clone());
      p_lats.push(p.location.lat);
      p_lngs.push(p.location.lng);
      p_dests.push(p.destination.clone());
    }

    let rows = Self::query_bearings(&state.db, line, &p_ids, &p_lats, &p_lngs, &p_dests).await;

    let bearing_map: HashMap<&str, f64> =
      rows.iter().filter_map(|r| r.bearing_deg.map(|b| (r.bus_id.as_str(), b))).collect();

    use crate::cache::BusTracking;
    for &i in &missing {
      let p = &mut positions[i];
      if let Some(&deg) = bearing_map.get(p.id.as_str()) {
        p.bearing_deg = Some(deg);
        state.cache.tracker().set_bearing(&p.id, deg).await;
      }
    }
  }

  async fn load_headsigns(state: &crate::State, route_short_name: &str) -> Headsigns {
    if let Some(cached) = Self::get_headsigns_cache(&state.cache, route_short_name).await {
      return cached;
    }

    let rows = Self::query_headsigns(&state.db, route_short_name).await;

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

    Self::set_headsigns_cache(&state.cache, route_short_name, &hs).await;
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
      let Some(line) = bus_positions.get(line_id) else { continue };
      for bus in &line.buses {
        if let Some(&idx) = best.get(&bus.id) {
          if bus.freshness == BusFreshness::Live && deduped[idx].freshness != BusFreshness::Live {
            deduped[idx] = bus.clone();
          }
        } else {
          best.insert(bus.id.clone(), deduped.len());
          deduped.push(bus.clone());
        }
      }
    }
    deduped
  }
}
