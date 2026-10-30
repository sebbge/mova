use super::bus::{Bus, BusFreshness, Line};
use super::*;
use crate::sdk::arrivals::{UpcomingArrival, UpcomingByLine};
use crate::sdk::vehicles::BusEntry;

mod repository;

pub type StopLines = HashMap<ServiceKey, StopLine>;

/// A line towards one destination
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceKey(String, String);

impl ServiceKey {
  pub fn new(line: &str, destination: Option<&str>) -> Self {
    Self(line.to_owned(), destination.map(|d| d.trim().to_uppercase()).unwrap_or_default())
  }
}

///# GET <MOVA>/stops/{stop_id}/arrivals
#[derive(Debug, Serialize, Deserialize)]
pub struct StopDetail {
  pub lines: Vec<StopLine>,
}

///# GET <MOVA>/stops/{stop_id}/schedule
/// Absolute times, cacheable until midnight
#[derive(Debug, Serialize, Deserialize)]
pub struct StopSchedule {
  pub service_date: String,
  pub lines: Vec<ScheduledLine>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduledLine {
  pub line: String,
  pub destination: Option<String>,
  /// s from midnight
  pub departures: Vec<i64>,
  /// Schedule from a zone marker
  pub zonal: bool,
  /// Interpolated between control points, only if all departures are
  pub interpolated: bool,
}

///# GET <MOVA>/stops/{stop_id}
/// GTFS only, prerendered by the web
#[derive(Debug, Serialize, Deserialize)]
pub struct Stop {
  pub stop_id: String,
  pub stop_name: String,
  /// Platforms only: station and number
  pub station: Option<String>,
  pub platform: Option<String>,
  /// Department of the point
  pub area: Option<String>,
  pub lat: f64,
  pub lng: f64,
  pub bearing_rad: Option<f64>,
  /// Refs of this same point, leader first
  pub siblings: Vec<String>,
  pub lines: Vec<StopRoute>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopRoute {
  pub line: String,
  /// By trip count
  pub destinations: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StopItem {
  pub id: String,
  pub name: String,
  pub lat: f64,
  pub lng: f64,
  /// Providers sharing the point, only when several
  pub providers: Option<Vec<String>>,
  pub station: Option<String>,
  pub platform: Option<String>,
  /// Terminal outline, `[lng, lat, lng, lat, …]`
  pub footprint: Option<Vec<f64>>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct StopHit {
  pub id: String,
  pub name: String,
  pub lat: f64,
  pub lng: f64,
  pub area: Option<String>,
  pub station: Option<String>,
  pub platform: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StopLine {
  pub line: String,
  pub destination: Option<String>,
  pub company: String,
  pub arrivals: Vec<StopArrival>,
}

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StopArrival {
  pub eta: Option<i64>,
  pub live: bool,
  pub bus: Option<Bus>,
  /// Zone marker schedules only
  pub zonal: bool,
  /// Scheduled only: interpolated time
  pub interpolated: bool,
  /// Another provider's bus placed here by its operator, no ETA
  pub stop_position: Option<StopPosition>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopPosition {
  At,
  Next,
}

//TODO: remove
pub struct ApproachingBusRow {
  pub line: String,
  pub headsign: Option<String>,
  pub bus_id: String,
  pub remaining_m: f64,
  pub scheduled_s: Option<f64>,
}

/// (shape position, s), backward stops dropped
pub struct ScheduleProfile(Vec<(f64, f64)>);

impl ScheduleProfile {
  pub fn new(fracs: &[f64], secs: &[f64]) -> Self {
    let mut knots: Vec<(f64, f64)> = Vec::with_capacity(fracs.len());
    for (&f, &t) in fracs.iter().zip(secs) {
      if knots.last().is_none_or(|&(lf, lt)| f >= lf && t >= lt) {
        knots.push((f, t));
      }
    }
    Self(knots)
  }

  fn at(&self, frac: f64) -> Option<f64> {
    let i = self.0.windows(2).position(|w| w[0].0 <= frac && frac <= w[1].0)?;
    let ((f0, t0), (f1, t1)) = (self.0[i], self.0[i + 1]);
    if f1 == f0 {
      return Some(t0);
    }
    Some(t0 + (frac - f0) / (f1 - f0) * (t1 - t0))
  }

  pub fn between(&self, from: f64, to: f64) -> Option<f64> {
    Some(self.at(to)? - self.at(from)?)
  }
}

//TODO: remove
pub struct PastStopBusRow {
  pub line: String,
  pub headsign: Option<String>,
  pub bus_id: String,
  pub past_m: f64,
}

impl StopArrival {
  const RT_GTFS_DEDUP_MARGIN_MIN: i64 = 6;

  /// Live and scheduled by ETA, schedules near a live bus dropped
  pub fn merge(arrivals: &mut Vec<Self>) {
    let rt_etas: Vec<i64> = arrivals.iter().filter(|a| a.live).filter_map(|a| a.eta).collect();
    arrivals.retain(|a| {
      a.live || a.eta.is_none_or(|eta| !rt_etas.iter().any(|&rt| (eta - rt).abs() <= Self::RT_GTFS_DEDUP_MARGIN_MIN))
    });
    arrivals.sort_by_key(Self::rank);
    arrivals.truncate(3);
  }

  /// By ETA, at-stop and next as now, live first on ties, untimed last
  fn rank(&self) -> (bool, i64, bool) {
    (self.eta.is_none() && self.stop_position.is_none(), self.eta.unwrap_or_default(), !self.live)
  }

  /// Commercial speed without schedule
  const FALLBACK_KMH: f64 = 15.0;
  const PLAUSIBLE_KMH: std::ops::RangeInclusive<f64> = 3.0..=90.0;
  const DEPARTED_KMH: f64 = 3.0;

  /// ETA from scheduled time between bus and stop
  pub fn from_approaching_bus(bus: &Bus, row: &ApproachingBusRow, had_eta: bool) -> Self {
    let waiting = !had_eta && bus.seen.speed.unwrap_or(0.0) < Self::DEPARTED_KMH;
    let eta = (!bus.still && !waiting).then(|| {
      let scheduled = row.scheduled_s.filter(|&s| Self::PLAUSIBLE_KMH.contains(&(row.remaining_m / s * 3.6)));
      let secs = scheduled.unwrap_or(row.remaining_m / (Self::FALLBACK_KMH / 3.6));
      (secs / 60.0).round() as i64
    });
    let eta = eta.filter(|&min| min <= 90);
    let mut snap = bus.clone();
    snap.past_stop = false;
    Self { eta, live: eta.is_some(), bus: Some(snap), zonal: false, interpolated: false, stop_position: None }
  }

  pub fn from_upcoming(u: &UpcomingArrival) -> Self {
    Self {
      eta: crate::helpers::seconds_to_mins(u.eta_s),
      live: true,
      zonal: false,
      interpolated: false,
      stop_position: None,
      bus: Some(Bus::live(BusEntry { timestamp: None, stop: None, next_stop: None, ..u.bus.clone() })),
    }
  }
}

impl StopLine {
  pub fn cmp_by_first_eta(&self, other: &Self) -> std::cmp::Ordering {
    let rank = |l: &Self| l.arrivals.first().map(StopArrival::rank).unwrap_or((true, 0, true));
    rank(self).cmp(&rank(other))
  }

  fn empty(line: String, destination: Option<String>) -> Self {
    Self { line, destination, company: String::new(), arrivals: vec![] }
  }

  pub fn key(&self) -> ServiceKey {
    ServiceKey::new(&self.line, self.destination.as_deref())
  }

  fn slot<'a>(lines: &'a mut StopLines, line: &str, destination: Option<&str>) -> &'a mut Self {
    lines
      .entry(ServiceKey::new(line, destination))
      .or_insert_with(|| Self::empty(line.to_owned(), destination.map(str::to_owned)))
  }

  async fn fuse(
    state: &crate::Transit,
    scope: &Scope,
    stop_id: &str,
    mut lines: StopLines,
    upcoming: UpcomingByLine,
    live: &HashMap<String, Line>,
  ) -> StopLines {
    // live upcoming, gtfs backfill up to 3 per line
    // row destination is the shape's GTFS headsign
    let shape_ids: Vec<String> =
      upcoming.values().flatten().filter_map(|u| u.bus.shape_id.clone()).collect::<HashSet<_>>().into_iter().collect();
    let headsigns = Self::shape_headsigns(&state.db, scope, &shape_ids).await.unwrap_or_default();
    let mut by_service: HashMap<ServiceKey, (String, Option<String>, Vec<StopArrival>)> = HashMap::new();
    for (line_id, buses) in upcoming {
      for u in &buses {
        let destination =
          u.bus.shape_id.as_ref().and_then(|s| headsigns.get(s)).cloned().or_else(|| u.bus.destination.clone());
        by_service
          .entry(ServiceKey::new(&line_id, destination.as_deref()))
          .or_insert_with(|| (line_id.clone(), destination, vec![]))
          .2
          .push(StopArrival::from_upcoming(u));
      }
    }
    for (key, (line_id, destination, mut arrivals)) in by_service {
      if let Some(existing) = lines.get(&key) {
        arrivals.extend(existing.arrivals.iter().cloned());
      }
      StopArrival::merge(&mut arrivals);

      let destination = lines.get(&key).and_then(|l| l.destination.clone()).or(destination);
      let company = lines.get(&key).map(|l| l.company.clone());

      lines.insert(key, StopLine { line: line_id, destination, company: company.unwrap_or_default(), arrivals });
    }

    // non-rt services from bus positions
    let covered: HashSet<ServiceKey> =
      lines.iter().filter(|(_, l)| l.arrivals.first().is_some_and(|a| a.live)).map(|(k, _)| k.clone()).collect();
    let claimed: HashSet<String> = lines
      .values()
      .flat_map(|l| &l.arrivals)
      .filter(|a| a.live)
      .filter_map(|a| a.bus.as_ref().map(|b| b.seen.id.clone()))
      .collect();
    let mut non_rt_ids: Vec<String> =
      lines.iter().filter(|(k, _)| !covered.contains(*k)).map(|(_, l)| l.line.clone()).collect();
    non_rt_ids.sort();
    non_rt_ids.dedup();

    if !non_rt_ids.is_empty() {
      let all_buses = Line::collect_deduped(&non_rt_ids, live);
      if !all_buses.is_empty() {
        let bus_meta: HashMap<String, &Bus> = all_buses.iter().map(|b| (b.seen.id.clone(), b)).collect();

        if let Ok(rows) = StopArrival::query_approaching(&state.db, scope, stop_id, &all_buses).await {
          use crate::cache::{BusTracking, bus_key};
          let ghosts = state.cache.tracker().stop_ghosts(scope.region.id(), stop_id).await;
          let mut computed: HashMap<ServiceKey, (String, Option<String>, Vec<StopArrival>)> = HashMap::new();
          for row in rows {
            let Some(bus) = bus_meta.get(&row.bus_id).filter(|_| !claimed.contains(&row.bus_id)) else {
              continue;
            };
            let destination = row.headsign.clone().or_else(|| bus.seen.destination.clone());
            let arrival =
              StopArrival::from_approaching_bus(bus, &row, ghosts.contains_key(&bus_key(scope.provider, &row.bus_id)));
            computed
              .entry(ServiceKey::new(&row.line, destination.as_deref()))
              .or_insert_with(|| (row.line.clone(), destination, vec![]))
              .2
              .push(arrival);
          }

          for (key, (line_id, destination, mut live_arrivals)) in computed {
            if covered.contains(&key) {
              continue;
            }
            StopArrival::merge(&mut live_arrivals);
            if live_arrivals.is_empty() {
              continue;
            }

            let stop_line = Self::slot(&mut lines, &line_id, destination.as_deref());
            let has_rt = live_arrivals.first().is_some_and(|a| a.live);

            if has_rt {
              live_arrivals.extend(stop_line.arrivals.iter().cloned());
              StopArrival::merge(&mut live_arrivals);
              stop_line.arrivals = live_arrivals;
            } else if let (Some(first_gtfs), Some(first_live)) =
              (stop_line.arrivals.first_mut(), live_arrivals.into_iter().next())
            {
              first_gtfs.bus = first_live.bus;
            }
          }
        }
      }
    }

    // rt metadata
    Self::enrich_snapshots(&mut lines, live);

    lines
  }

  // ghost tracking

  /// Keeps rt arrivals across feed gaps
  async fn apply_ghosts(
    state: &crate::Transit,
    scope: &Scope,
    stop_id: &str,
    lines: &mut StopLines,
    positions: &HashMap<String, Line>,
  ) {
    use crate::cache::{BusTracking, StopGhost, bus_key, split_bus_key};
    let tracker = state.cache.tracker();
    let (region, provider) = (scope.region.id(), scope.provider);
    let mut ghost_map = tracker.stop_ghosts(region, stop_id).await;

    let live_bus_ids: HashSet<String> = positions
      .values()
      .flat_map(|l| l.buses.iter())
      .filter(|b| b.freshness == BusFreshness::Live)
      .map(|b| bus_key(provider, &b.seen.id))
      .collect();

    let current_rt_bus_ids: HashSet<String> = lines
      .values()
      .flat_map(|l| &l.arrivals)
      .filter(|a| a.live)
      .filter_map(|a| a.bus.as_ref().map(|b| bus_key(provider, &b.seen.id)))
      .collect();

    for line_data in lines.values() {
      for arrival in &line_data.arrivals {
        if !arrival.live {
          continue;
        }
        let Some(bus) = arrival.bus.as_ref() else {
          continue;
        };
        tracker.mark_seen(provider, &bus.seen.id).await;
        let key = bus_key(provider, &bus.seen.id);
        let ghost = StopGhost {
          arrival: arrival.clone(),
          line: line_data.line.clone(),
          destination: line_data.destination.clone(),
        };
        tracker.set_stop_ghost(region, stop_id, &key, ghost.clone()).await;
        ghost_map.insert(key, ghost);
      }
    }

    let ghost_ids: Vec<String> =
      ghost_map.keys().filter(|id| !current_rt_bus_ids.contains(id.as_str())).cloned().collect();

    let mut ghost_count = 0usize;
    for id in ghost_ids {
      if live_bus_ids.contains(&id) {
        tracker.remove_stop_ghost(region, stop_id, &id).await;
        continue;
      }

      let Some((prov, raw)) = split_bus_key(&id) else {
        continue;
      };

      let Some((freshness, last_seen_epoch)) = tracker.missing(prov, raw).await else {
        tracker.remove_stop_ghost(region, stop_id, &id).await;
        continue;
      };

      let Some(entry) = ghost_map.get(&id) else { continue };

      let now_epoch = chrono::Utc::now().timestamp();
      let elapsed_min = (now_epoch - last_seen_epoch).max(0) / 60;
      let adjusted_eta = entry.arrival.eta.map(|m| m - elapsed_min);

      if adjusted_eta.is_some_and(|m| m < 0) {
        tracker.remove_stop_ghost(region, stop_id, &id).await;
        tracker.remove(prov, raw).await;
        continue;
      }

      let mut ghost_arrival = entry.arrival.clone();
      ghost_arrival.eta = adjusted_eta;
      if let Some(snap) = ghost_arrival.bus.as_mut() {
        snap.freshness = freshness;
      }

      ghost_count += 1;
      Self::slot(lines, &entry.line, entry.destination.as_deref()).arrivals.push(ghost_arrival);
    }

    if ghost_count > 0 {
      for stop_line in lines.values_mut() {
        StopArrival::merge(&mut stop_line.arrivals);
      }
      tracing::info!(stop = %stop_id, ghosts = ghost_count, "[stops] ghost tracking injected");
    }
  }

  fn enrich_snapshots(lines: &mut StopLines, positions: &HashMap<String, Line>) {
    for stop_line in lines.values_mut() {
      let line_buses = match positions.get(&stop_line.line) {
        Some(l) if !l.buses.is_empty() => &l.buses,
        _ => continue,
      };

      for arrival in &mut stop_line.arrivals {
        let Some(snap) = arrival.bus.as_mut() else {
          continue;
        };

        let best = line_buses.iter().find(|b| b.seen.id == snap.seen.id).or_else(|| {
          let (slat, slng) = (snap.seen.location.lat, snap.seen.location.lng);
          line_buses.iter().min_by(|a, b| {
            let da = (a.seen.location.lat - slat).powi(2) + (a.seen.location.lng - slng).powi(2);
            let db = (b.seen.location.lat - slat).powi(2) + (b.seen.location.lng - slng).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
          })
        });

        if let Some(bus) = best {
          snap.bearing_deg = snap.bearing_deg.or(bus.bearing_deg);
          snap.headsign = snap.headsign.clone().or(bus.headsign.clone());
          snap.still = bus.still;
          snap.seen.speed = bus.seen.speed;
          if bus.freshness != BusFreshness::Live {
            snap.freshness = bus.freshness;
          }
        }
      }
    }
  }

  // past-stop buses

  /// Buses up to 500 m past the stop
  async fn attach_past_stop_buses(
    state: &crate::Transit,
    scope: &Scope,
    stop_id: &str,
    lines: &mut StopLines,
    positions: &HashMap<String, Line>,
  ) {
    const PAST_STOP_MARGIN_M: f64 = 500.0;

    let mut line_ids: Vec<String> = lines.values().map(|l| l.line.clone()).collect();
    line_ids.sort();
    line_ids.dedup();
    let all_buses = Line::collect_deduped(&line_ids, positions);
    if all_buses.is_empty() {
      return;
    }

    let bus_meta: HashMap<String, &Bus> = all_buses.iter().map(|b| (b.seen.id.clone(), b)).collect();

    let rows = match StopArrival::query_past_stop(&state.db, scope, stop_id, &all_buses).await {
      Ok(rows) => rows,
      Err(_) => return,
    };

    let known_bus_ids: HashSet<String> = lines
      .values()
      .flat_map(|l| l.arrivals.iter())
      .filter_map(|a| a.bus.as_ref().map(|b| b.seen.id.clone()))
      .collect();

    for row in rows {
      if row.past_m > PAST_STOP_MARGIN_M {
        continue;
      }
      let Some(bus) = bus_meta.get(&row.bus_id) else {
        continue;
      };
      if known_bus_ids.contains(&bus.seen.id) {
        continue;
      }
      let mut snap = (*bus).clone();
      snap.past_stop = true;

      let destination = row.headsign.or_else(|| bus.seen.destination.clone());
      Self::slot(lines, &row.line, destination.as_deref()).arrivals.push(StopArrival {
        eta: None,
        live: false,
        bus: Some(snap),
        zonal: false,
        interpolated: false,
        stop_position: None,
      });
    }
  }

  /// Shapeless lines: buses without ETA, map only
  async fn attach_shapeless(
    state: &crate::Transit,
    scope: &Scope,
    lines: &mut StopLines,
    positions: &HashMap<String, Line>,
  ) {
    let mut known: HashSet<String> = lines
      .values()
      .flat_map(|l| l.arrivals.iter())
      .filter_map(|a| a.bus.as_ref().map(|b| b.seen.id.clone()))
      .collect();

    let served: HashSet<&str> = lines.values().map(|l| l.line.as_str()).collect();
    let mut loose: Vec<(String, Bus)> = Vec::new();
    for (line_id, line) in positions.iter().filter(|(id, _)| served.contains(id.as_str())) {
      for bus in &line.buses {
        if known.insert(bus.seen.id.clone()) {
          loose.push((line_id.clone(), bus.clone()));
        }
      }
    }
    if loose.is_empty() {
      return;
    }

    let mut candidates: Vec<String> = loose.iter().map(|(id, _)| id.clone()).collect();
    candidates.sort();
    candidates.dedup();
    let Ok(shapeless) = Self::shapeless(&state.db, scope, &candidates).await else {
      return;
    };

    for (line_id, bus) in loose.into_iter().filter(|(id, _)| shapeless.contains(id)) {
      let destination = bus.seen.destination.clone();
      Self::slot(lines, &line_id, destination.as_deref()).arrivals.push(StopArrival {
        eta: None,
        live: false,
        bus: Some(bus),
        zonal: false,
        interpolated: false,
        stop_position: None,
      });
    }
  }
}

use super::bus::Lines;
use crate::scope::Scope;
use crate::sdk::vehicles::Vehicles;

impl StopLine {
  /// Same-authority buses placed here by their operator, name checked against code
  async fn attach_relayed(state: &crate::Transit, scope: &Scope, stop_id: &str, lines: &mut StopLines) {
    use crate::cache::BusTracking;

    let others: Vec<Scope> = state
      .scopes()
      .filter(|s| s.region == scope.region && s.provider != scope.provider)
      .filter(|s| matches!(s.vehicles(), Some(Vehicles::Fleet(_))))
      .collect();
    if others.is_empty() {
      return;
    }
    let name = match Stop::name(&state.db, scope, stop_id).await {
      Ok(Some(name)) => match_key(&name),
      Ok(None) => return,
      Err(e) => {
        tracing::warn!(error = ?e, "[stops] relay skipped");
        return;
      }
    };

    let tracker = state.cache.tracker();
    let is_here = |stop: &Option<crate::sdk::vehicles::StopRef>| {
      stop.as_ref().is_some_and(|p| p.id == stop_id && match_key(&p.name) == name)
    };
    for other in others {
      tracker.touch(other.provider);
      let Some(view) = tracker.view(other.provider, Line::FLEET_STALE).await else {
        continue;
      };
      for bus in view.by_line.values().flatten() {
        let position = match (is_here(&bus.seen.stop), is_here(&bus.seen.next_stop)) {
          (true, _) => StopPosition::At,
          (_, true) => StopPosition::Next,
          _ => continue,
        };
        let arrival = StopArrival {
          eta: None,
          live: true,
          bus: Some(bus.clone()),
          zonal: false,
          interpolated: false,
          stop_position: Some(position),
        };
        Self::slot(lines, &bus.seen.line, bus.seen.destination.as_deref()).arrivals.push(arrival);
      }
    }
  }
}

fn match_key(raw: &str) -> String {
  raw.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}
impl StopDetail {
  pub async fn assemble(state: &crate::Transit, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    // gtfs departures
    let static_lines = StopLine::get(&state.db, scope, stop_id).await?;
    let mut line_ids: Vec<String> = static_lines.values().map(|l| l.line.clone()).collect();
    line_ids.sort();
    line_ids.dedup();

    // provider arrivals + line buses
    let upcoming = async {
      match scope.arrivals() {
        Some(arrivals) => arrivals.upcoming(stop_id, &line_ids).await.unwrap_or_default(),
        None => Default::default(),
      }
    };
    let (upcoming_lines, live_lines) = tokio::join!(upcoming, Line::fetch(state, scope, &line_ids));

    // merge
    Ok(Self::build(state, scope, stop_id, static_lines, upcoming_lines, live_lines.unwrap_or_default()).await)
  }

  pub async fn build(
    state: &crate::Transit,
    scope: &Scope,
    stop_id: &str,
    static_lines: StopLines,
    upcoming: UpcomingByLine,
    positions: Lines,
  ) -> Self {
    let mut lines = StopLine::fuse(state, scope, stop_id, static_lines, upcoming, &positions).await;
    StopLine::apply_ghosts(state, scope, stop_id, &mut lines, &positions).await;
    StopLine::attach_past_stop_buses(state, scope, stop_id, &mut lines, &positions).await;
    StopLine::attach_shapeless(state, scope, &mut lines, &positions).await;
    StopLine::attach_relayed(state, scope, stop_id, &mut lines).await;
    Self::from_lines(lines)
  }

  fn from_lines(line_map: StopLines) -> Self {
    let with_arrivals: HashSet<String> =
      line_map.values().filter(|l| !l.arrivals.is_empty()).map(|l| l.line.clone()).collect();
    let mut lines: Vec<StopLine> = line_map
      .into_values()
      // roster line without arrivals dropped if another row has them
      .filter(|l| !l.arrivals.is_empty() || !with_arrivals.contains(&l.line))
      .map(|mut l| {
        // sort what was added after merging
        l.arrivals.sort_by_key(StopArrival::rank);
        let bus = l.arrivals.first().and_then(|a| a.bus.as_ref());
        if let Some(b) = bus.filter(|b| !b.seen.company.is_empty()) {
          l.company.clone_from(&b.seen.company);
        }
        // roster lines take the bus destination
        l.destination = l.destination.or_else(|| bus.and_then(|b| b.seen.destination.clone()));
        l
      })
      .collect();

    lines.sort_by(|a, b| a.cmp_by_first_eta(b));

    let detail = Self { lines };

    #[cfg(debug_assertions)] // DEBUG: write response to response.json
    if let Ok(json) = serde_json::to_string_pretty(&detail)
      && let Err(e) = std::fs::write("../response.json", &json)
    {
      tracing::warn!(error = ?e, "[stops] failed to write debug response.json");
    }

    detail
  }
}
