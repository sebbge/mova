use super::{Grid, Pattern, Stop, Time, Timetable, Transfer, Trip, Walking};
use crate::fare::FareCatalog;
use mova_sdk::fares::FareProduct;
use mova_transit::entities::service_day::ServiceDay;

use rustc_hash::FxHashMap;

impl Timetable {
  const TRANSFER_RADIUS_M: f64 = 400.0;

  pub fn build(day: ServiceDay, fares: Vec<FareProduct>) -> Self {
    let ServiceDay { date, stops: day_stops, trips: day_trips } = day;

    let (stops, clusters): (Vec<Stop>, Vec<String>) =
      day_stops.into_iter().map(|s| (Stop { id: s.id, name: s.name, location: s.location }, s.cluster)).unzip();

    let mut trips = Vec::with_capacity(day_trips.len());
    let mut groups: FxHashMap<Vec<u32>, Vec<(u32, Vec<Time>)>> = FxHashMap::default();
    for trip in day_trips {
      let sequence: Vec<u32> = trip.stop_times.iter().map(|st| st.stop).collect();
      let times = trip
        .stop_times
        .iter()
        .map(|st| Time { arrival: st.arrival, departure: st.departure.max(st.arrival), interpolated: st.interpolated })
        .collect();
      groups.entry(sequence).or_default().push((trips.len() as u32, times));
      trips.push(Trip { route: trip.route, line: trip.line, headsign: trip.headsign, shape_id: trip.shape_id });
    }

    let mut patterns = Vec::new();
    for (sequence, mut members) in groups {
      members.sort_by_key(|(_, times)| times[0].departure);
      let n = sequence.len();
      let mut fifo: Vec<(Vec<u32>, Vec<Time>)> = Vec::new();
      for (trip, times) in members {
        let fits = fifo.iter().position(|(_, flat)| {
          flat[flat.len() - n..].iter().zip(&times).all(|(a, b)| a.arrival <= b.arrival && a.departure <= b.departure)
        });
        match fits {
          Some(i) => {
            fifo[i].0.push(trip);
            fifo[i].1.extend(times);
          }
          None => fifo.push((vec![trip], times)),
        }
      }
      patterns.extend(fifo.into_iter().map(|(trips, times)| Pattern { stops: sequence.clone(), trips, times }));
    }

    let mut stop_patterns: Vec<Vec<(u32, u32)>> = vec![Vec::new(); stops.len()];
    for (p, pattern) in patterns.iter().enumerate() {
      for (pos, &s) in pattern.stops.iter().enumerate() {
        stop_patterns[s as usize].push((p as u32, pos as u32));
      }
    }

    let grid = Grid::new(stops.iter().map(|s| s.location));
    let transfers = (0..stops.len())
      .map(|s| {
        if stop_patterns[s].is_empty() {
          return Vec::new();
        }
        let from = stops[s].location;
        grid
          .around(from, Self::TRANSFER_RADIUS_M)
          .filter(|&t| t as usize != s && !stop_patterns[t as usize].is_empty())
          .filter_map(|t| {
            let to = stops[t as usize].location;
            if clusters[s] == clusters[t as usize] {
              return Some(Transfer { to: t, secs: Self::MIN_CHANGE_S, meters: from.distance_to(&to) });
            }
            (from.distance_to(&to) <= Self::TRANSFER_RADIUS_M).then(|| {
              let w = Walking::between(from, to);
              Transfer { to: t, secs: w.secs + Self::MIN_CHANGE_S, meters: w.meters }
            })
          })
          .collect()
      })
      .collect();

    Self { date, stops, trips, patterns, stop_patterns, transfers, fares: FareCatalog::new(fares), grid }
  }
}
