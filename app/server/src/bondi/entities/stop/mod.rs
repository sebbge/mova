use super::bus::{Bus, BusFreshness, Line};
use super::*;
use crate::im_api::models::ImUpcomingBus;

mod repository;

pub type StopLines = HashMap<String, StopLine>;
pub type ImUpcomingLines = HashMap<String, Vec<ImUpcomingBus>>;

///# GET <BONDI>/stops/{stop_id}
#[derive(Debug, Serialize, Deserialize)]
pub struct StopDetail {
  pub lines: Vec<StopLine>,
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
}

//TODO: remove
pub struct ApproachingBusRow {
  pub line: String,
  pub bus_id: String,
  pub remaining_m: f64,
}

//TODO: remove
pub struct PastStopBusRow {
  pub line: String,
  pub bus_id: String,
  pub past_m: f64,
}

impl StopArrival {
  const RT_GTFS_DEDUP_MARGIN_MIN: i64 = 3;

  pub fn dedup_gtfs_backfill(rt_arrivals: &[Self], gtfs_arrivals: impl Iterator<Item = Self>) -> Vec<Self> {
    let rt_etas: Vec<i64> = rt_arrivals.iter().filter(|a| a.live).filter_map(|a| a.eta).collect();
    gtfs_arrivals
      .filter(|a| {
        let Some(eta) = a.eta else { return true };
        !rt_etas.iter().any(|&rt| (eta - rt).abs() <= Self::RT_GTFS_DEDUP_MARGIN_MIN)
      })
      .collect()
  }

  pub fn sort_by_eta(arrivals: &mut [Self]) {
    arrivals.sort_by(|a, b| match (a.eta, b.eta) {
      (Some(x), Some(y)) => x.cmp(&y),
      (Some(_), None) => std::cmp::Ordering::Less,
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (None, None) => std::cmp::Ordering::Equal,
    });
  }

  pub fn from_approaching_bus(bus: &Bus, remaining_m: f64) -> Self {
    let speed = bus.get_speed();
    let eta = if speed >= 3.0 {
      let min = (remaining_m / (speed * 1000.0 / 3600.0) / 60.0).round() as i64;
      if min <= 90 { Some(min) } else { None }
    } else {
      None
    };
    let mut snap = bus.clone();
    snap.past_stop = false;
    Self { eta, live: eta.is_some(), bus: Some(snap) }
  }

  pub fn from_upcoming(u: &crate::im_api::models::ImUpcomingBus) -> Self {
    Self {
      eta: crate::helpers::seconds_to_mins(u.eta),
      live: true,
      bus: Some(Bus {
        id: u.bus.bus_id.to_string(),
        line: u.bus.line.clone(),
        location: u.bus.location.clone(),
        bearing_deg: None,
        freshness: BusFreshness::Live,
        origin: Some(u.bus.origin.clone()),
        destination: Some(u.bus.destination.clone()),
        company: u.bus.company.clone(),
        headsign: None,
        timestamp: None,
        still: false,
        speed: 0.0,
        past_stop: false,
        access: u.bus.access.label(),
        emissions: u.bus.emissions.label(),
        thermal_comfort: u.bus.thermal_confort.label(),
        stale_count: 0,
        last_seen_epoch: chrono::Utc::now().timestamp(),
      }),
    }
  }
}

impl StopLine {
  pub fn cmp_by_first_eta(&self, other: &Self) -> std::cmp::Ordering {
    let ea = self.arrivals.first().and_then(|a| a.eta);
    let eb = other.arrivals.first().and_then(|a| a.eta);
    ea.cmp(&eb)
  }

  fn empty(line: String) -> Self {
    Self { line, destination: None, company: String::new(), arrivals: vec![] }
  }

  async fn fuse(
    state: &crate::State,
    stop_id: &str,
    mut lines: StopLines,
    upcoming: ImUpcomingLines,
    live: &HashMap<String, Line>,
  ) -> StopLines {
    // live upcoming, gtfs backfill up to 3 per line
    for (line_id, buses) in upcoming {
      let mut arrivals: Vec<StopArrival> = buses.iter().map(StopArrival::from_upcoming).collect();

      let remaining = 3_usize.saturating_sub(arrivals.len());
      if remaining > 0
        && let Some(existing) = lines.get(&line_id)
      {
        let deduped = StopArrival::dedup_gtfs_backfill(&arrivals, existing.arrivals.iter().cloned());
        arrivals.extend(deduped.into_iter().take(remaining));
      }
      arrivals.truncate(3);

      let destination = lines.get(&line_id).and_then(|l| l.destination.clone());
      let company = lines.get(&line_id).map(|l| l.company.clone());

      lines.insert(
        line_id.clone(),
        StopLine { line: line_id, destination, company: company.unwrap_or_default(), arrivals },
      );
    }

    // non-rt lines from bus positions
    let non_rt_ids: Vec<String> =
      lines.iter().filter(|(_, l)| !l.arrivals.first().is_some_and(|a| a.live)).map(|(k, _)| k.clone()).collect();

    if !non_rt_ids.is_empty() {
      let all_buses = Line::collect_deduped(&non_rt_ids, live);
      if !all_buses.is_empty() {
        let bus_meta: HashMap<String, &Bus> = all_buses.iter().map(|b| (b.id.clone(), b)).collect();

        if let Ok(rows) = StopArrival::query_approaching(&state.db, stop_id, &all_buses).await {
          let mut computed: HashMap<String, Vec<StopArrival>> = HashMap::new();
          for row in rows {
            let Some(bus) = bus_meta.get(&row.bus_id) else { continue };
            computed.entry(row.line.clone()).or_default().push(StopArrival::from_approaching_bus(bus, row.remaining_m));
          }

          for (line_id, mut live_arrivals) in computed {
            StopArrival::sort_by_eta(&mut live_arrivals);
            live_arrivals.truncate(3);
            if live_arrivals.is_empty() {
              continue;
            }

            let stop_line = lines.entry(line_id.clone()).or_insert_with(|| Self::empty(line_id));
            let has_rt = live_arrivals.first().is_some_and(|a| a.live);

            if has_rt {
              let remaining = 3_usize.saturating_sub(live_arrivals.len());
              let deduped = StopArrival::dedup_gtfs_backfill(&live_arrivals, stop_line.arrivals.iter().cloned());
              live_arrivals.extend(deduped.into_iter().take(remaining));
              live_arrivals.truncate(3);
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
  async fn apply_ghosts(state: &crate::State, stop_id: &str, lines: &mut StopLines, positions: &HashMap<String, Line>) {
    use crate::cache::{BusTracking, StopGhost};
    let tracker = state.cache.tracker();
    let mut ghost_map = tracker.stop_ghosts(stop_id).await;

    let live_bus_ids: HashSet<String> = positions
      .values()
      .flat_map(|l| l.buses.iter())
      .filter(|b| b.freshness == BusFreshness::Live)
      .map(|b| b.id.clone())
      .collect();

    let current_rt_bus_ids: HashSet<String> = lines
      .values()
      .flat_map(|l| &l.arrivals)
      .filter(|a| a.live)
      .filter_map(|a| a.bus.as_ref().map(|b| b.id.clone()))
      .collect();

    for line_data in lines.values() {
      for arrival in &line_data.arrivals {
        if !arrival.live {
          continue;
        }
        let Some(bus) = arrival.bus.as_ref() else { continue };
        tracker.mark_seen(&bus.id).await;
        let ghost = StopGhost { arrival: arrival.clone(), line: line_data.line.clone() };
        tracker.set_stop_ghost(stop_id, &bus.id, ghost.clone()).await;
        ghost_map.insert(bus.id.clone(), ghost);
      }
    }

    let ghost_ids: Vec<String> =
      ghost_map.keys().filter(|id| !current_rt_bus_ids.contains(id.as_str())).cloned().collect();

    let mut ghost_count = 0usize;
    for id in ghost_ids {
      if live_bus_ids.contains(&id) {
        tracker.remove_stop_ghost(stop_id, &id).await;
        continue;
      }

      let Some((freshness, last_seen_epoch)) = tracker.missing(&id).await else {
        tracker.remove_stop_ghost(stop_id, &id).await;
        continue;
      };

      let entry = ghost_map.get(&id).unwrap();

      let now_epoch = chrono::Utc::now().timestamp();
      let elapsed_min = (now_epoch - last_seen_epoch).max(0) / 60;
      let adjusted_eta = entry.arrival.eta.map(|m| m - elapsed_min);

      if adjusted_eta.is_some_and(|m| m < 0) {
        tracker.remove_stop_ghost(stop_id, &id).await;
        tracker.remove(&id).await;
        continue;
      }

      let mut ghost_arrival = entry.arrival.clone();
      ghost_arrival.eta = adjusted_eta;
      if let Some(snap) = ghost_arrival.bus.as_mut() {
        snap.freshness = freshness;
      }

      ghost_count += 1;
      let stop_line = lines.entry(entry.line.clone()).or_insert_with(|| Self::empty(entry.line.clone()));
      stop_line.arrivals.push(ghost_arrival);
    }

    if ghost_count > 0 {
      for stop_line in lines.values_mut() {
        StopArrival::sort_by_eta(&mut stop_line.arrivals);
        stop_line.arrivals.truncate(3);
      }
      tracing::info!(stop = %stop_id, ghosts = ghost_count, "stop ghost tracking injected");
    }
  }

  fn enrich_snapshots(lines: &mut StopLines, positions: &HashMap<String, Line>) {
    let line_ids: Vec<String> =
      lines.iter().filter(|(_, l)| l.arrivals.iter().any(|a| a.bus.is_some())).map(|(k, _)| k.clone()).collect();

    for line in line_ids {
      let line_buses = match positions.get(&line) {
        Some(l) if !l.buses.is_empty() => &l.buses,
        _ => continue,
      };

      let Some(stop_line) = lines.get_mut(&line) else { continue };
      for arrival in &mut stop_line.arrivals {
        let Some(snap) = arrival.bus.as_mut() else { continue };

        let best = line_buses.iter().find(|b| b.id == snap.id).or_else(|| {
          let (slat, slng) = (snap.location.lat, snap.location.lng);
          line_buses.iter().min_by(|a, b| {
            let da = (a.location.lat - slat).powi(2) + (a.location.lng - slng).powi(2);
            let db = (b.location.lat - slat).powi(2) + (b.location.lng - slng).powi(2);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
          })
        });

        if let Some(bus) = best {
          snap.bearing_deg = snap.bearing_deg.or(bus.bearing_deg);
          snap.headsign = snap.headsign.clone().or(bus.headsign.clone());
          snap.still = bus.still;
          snap.speed = bus.speed;
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
    state: &crate::State,
    stop_id: &str,
    lines: &mut StopLines,
    positions: &HashMap<String, Line>,
  ) {
    const PAST_STOP_MARGIN_M: f64 = 500.0;

    let line_ids: Vec<String> = lines.keys().cloned().collect();
    let all_buses = Line::collect_deduped(&line_ids, positions);
    if all_buses.is_empty() {
      return;
    }

    let bus_meta: HashMap<String, &Bus> = all_buses.iter().map(|b| (b.id.clone(), b)).collect();

    let rows = match StopArrival::query_past_stop(&state.db, stop_id, &all_buses).await {
      Ok(rows) => rows,
      Err(_) => return,
    };

    let known_bus_ids: HashSet<String> =
      lines.values().flat_map(|l| l.arrivals.iter()).filter_map(|a| a.bus.as_ref().map(|b| b.id.clone())).collect();

    for row in rows {
      if row.past_m > PAST_STOP_MARGIN_M {
        continue;
      }
      let Some(bus) = bus_meta.get(&row.bus_id) else { continue };
      if known_bus_ids.contains(&bus.id) {
        continue;
      }
      let mut snap = (*bus).clone();
      snap.past_stop = true;

      let stop_line = lines.entry(row.line.clone()).or_insert_with(|| Self::empty(row.line.clone()));
      stop_line.arrivals.push(StopArrival { eta: None, live: false, bus: Some(snap) });
    }
  }
}

use super::bus::Lines;
impl StopDetail {
  pub async fn build(
    state: &crate::State,
    stop_id: &str,
    static_lines: StopLines,
    upcoming: ImUpcomingLines,
    positions: Lines,
  ) -> Self {
    let mut lines = StopLine::fuse(state, stop_id, static_lines, upcoming, &positions).await;
    StopLine::apply_ghosts(state, stop_id, &mut lines, &positions).await;
    StopLine::attach_past_stop_buses(state, stop_id, &mut lines, &positions).await;
    Self::from_lines(lines)
  }

  fn from_lines(line_map: StopLines) -> Self {
    let mut lines: Vec<StopLine> = line_map
      .into_values()
      .map(|mut l| {
        l.company = l.arrivals.first().and_then(|a| a.bus.as_ref().map(|b| b.company.clone())).unwrap_or_default();
        l
      })
      .collect();

    lines.sort_by(|a, b| a.cmp_by_first_eta(b));

    let detail = Self { lines };

    // DEBUG: write response to response.json
    if let Ok(json) = serde_json::to_string_pretty(&detail)
      && let Err(e) = std::fs::write("../response.json", &json)
    {
      tracing::warn!(error = ?e, "failed to write debug response.json");
    }

    detail
  }
}
