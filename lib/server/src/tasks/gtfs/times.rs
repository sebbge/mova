use crate::core::geo::Point;

use gtfs_structures::StopTime;

pub(super) struct Timing {
  pub arrival: Option<u32>,
  pub departure: Option<u32>,
  pub interpolated: bool,
}

/// Times interpolated by distance between control points
pub(super) fn fill(stop_times: &[StopTime]) -> Vec<Option<Timing>> {
  let dist = distances(stop_times);
  let anchors: Vec<usize> =
    (0..stop_times.len()).filter(|&i| stop_times[i].arrival_time.or(stop_times[i].departure_time).is_some()).collect();

  let mut out: Vec<Option<Timing>> = stop_times
    .iter()
    .map(|st| {
      let (arrival, departure) = (st.arrival_time.or(st.departure_time), st.departure_time.or(st.arrival_time));
      arrival.map(|_| Timing { arrival, departure, interpolated: false })
    })
    .collect();

  for pair in anchors.windows(2) {
    let (a, b) = (pair[0], pair[1]);
    let (Some(from), Some(to)) = (
      stop_times[a].departure_time.or(stop_times[a].arrival_time),
      stop_times[b].arrival_time.or(stop_times[b].departure_time),
    ) else {
      continue;
    };
    let span = dist[b] - dist[a];
    for (i, slot) in out.iter_mut().enumerate().take(b).skip(a + 1) {
      let share = if span > 0.0 { (dist[i] - dist[a]) / span } else { (i - a) as f64 / (b - a) as f64 };
      let t = from + (f64::from(to.saturating_sub(from)) * share).round() as u32;
      *slot = Some(Timing { arrival: Some(t), departure: Some(t), interpolated: true });
    }
  }
  out
}

/// Distance per stop: the feed's, else straight-line sum
fn distances(stop_times: &[StopTime]) -> Vec<f64> {
  if let Some(fed) = stop_times.iter().map(|st| st.shape_dist_traveled.map(f64::from)).collect::<Option<Vec<_>>>() {
    return fed;
  }

  let mut out = Vec::with_capacity(stop_times.len());
  let (mut acc, mut prev): (f64, Option<Point>) = (0.0, None);
  for st in stop_times {
    let here = match (st.stop.latitude, st.stop.longitude) {
      (Some(lat), Some(lng)) if lat != 0.0 && lng != 0.0 => Some(Point { lat, lng }),
      _ => None,
    };
    if let Some(p) = here {
      if let Some(q) = prev {
        acc += q.distance_to(&p);
      }
      prev = Some(p);
    }
    out.push(acc);
  }
  out
}

#[cfg(test)]
mod tests {
  use super::fill;

  use gtfs_structures::{Stop, StopTime};
  use std::sync::Arc;

  fn st(seq: u32, lat: f64, time: Option<u32>) -> StopTime {
    StopTime {
      arrival_time: time,
      departure_time: time,
      stop: Arc::new(Stop { latitude: Some(lat), longitude: Some(-56.0), ..Default::default() }),
      stop_sequence: seq,
      ..Default::default()
    }
  }

  #[test]
  fn spreads_by_distance_between_control_points() {
    // 0 and 3 timed, 1 at a third, 2 near the end
    let trip = [st(1, -34.0, Some(1000)), st(2, -34.01, None), st(3, -34.028, None), st(4, -34.03, Some(1300))];
    let timings = fill(&trip);

    let t: Vec<Option<u32>> = timings.iter().map(|h| h.as_ref().and_then(|h| h.departure)).collect();
    assert_eq!(t, [Some(1000), Some(1100), Some(1280), Some(1300)]);
    let marked: Vec<bool> = timings.iter().map(|h| h.as_ref().is_some_and(|h| h.interpolated)).collect();
    assert_eq!(marked, [false, true, true, false]);
  }

  #[test]
  fn invents_nothing_outside_control_points() {
    let trip = [st(1, -34.0, None), st(2, -34.01, Some(1000)), st(3, -34.02, Some(1100)), st(4, -34.03, None)];
    let timings = fill(&trip);
    assert!(timings[0].is_none() && timings[3].is_none());
  }

  #[test]
  fn without_coordinates_spreads_evenly() {
    let mut trip = [st(1, -34.0, Some(1000)), st(2, 0.0, None), st(3, 0.0, None), st(4, -34.0, Some(1300))];
    for s in &mut trip[1..3] {
      s.stop = Arc::new(Stop::default());
    }
    let t: Vec<Option<u32>> = fill(&trip).iter().map(|h| h.as_ref().and_then(|h| h.departure)).collect();
    assert_eq!(t, [Some(1000), Some(1100), Some(1200), Some(1300)]);
  }
}
