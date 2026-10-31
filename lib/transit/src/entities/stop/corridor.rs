//! Span travel time measured from the recent trails of buses

use crate::cache::TrailPoint;
use crate::scope::Scope;
use crate::sdk::geo::Point;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

/// Farther from the shape is another street
const ON_ROUTE_M: f64 = 30.0;
/// Farther back is another pass of the shape
const BACK_M: f64 = 20.0;
/// Piece length, median per piece
const BIN_M: f64 = 100.0;
/// Min covered fraction, else the schedule
const MIN_COVERAGE: f64 = 0.5;
/// Min buses per piece
const MIN_BUSES: usize = 2;
/// Max trail age
const WINDOW_S: i64 = 900;
/// GPS jump above this speed
const MAX_MPS: f64 = 25.0;
/// Max gap between readings
const MAX_GAP_S: i64 = 600;

/// Shape in meters with cumulative distance per vertex
pub struct Polyline {
  lat0: f64,
  lng0: f64,
  kx: f64,
  xy: Vec<(f64, f64)>,
  cum: Vec<f64>,
}

impl Polyline {
  fn new(coords: &[(f64, f64)]) -> Option<Self> {
    let &(lng0, lat0) = coords.first()?;
    let kx = 111_320.0 * lat0.to_radians().cos();
    let xy: Vec<(f64, f64)> = coords.iter().map(|&(lng, lat)| ((lng - lng0) * kx, (lat - lat0) * 110_540.0)).collect();
    let mut cum = Vec::with_capacity(xy.len());
    let mut total = 0.0;
    for (i, p) in xy.iter().enumerate() {
      if i > 0 {
        total += (p.0 - xy[i - 1].0).hypot(p.1 - xy[i - 1].1);
      }
      cum.push(total);
    }
    (xy.len() >= 2).then_some(Self { lat0, lng0, kx, xy, cum })
  }

  /// Positions near the point, one per pass
  fn candidates(&self, p: &Point) -> Vec<f64> {
    let (px, py) = ((p.lng - self.lng0) * self.kx, (p.lat - self.lat0) * 110_540.0);
    let mut found: Vec<(f64, f64)> = Vec::new();
    for i in 0..self.xy.len() - 1 {
      let ((ax, ay), (bx, by)) = (self.xy[i], self.xy[i + 1]);
      let (dx, dy) = (bx - ax, by - ay);
      let len2 = dx * dx + dy * dy;
      let u = if len2 > 0.0 { (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
      let off = (ax + u * dx - px).hypot(ay + u * dy - py);
      if off > ON_ROUTE_M {
        continue;
      }
      let along = self.cum[i] + u * len2.sqrt();
      match found.last_mut() {
        Some(last) if along - last.0 < 2.0 * ON_ROUTE_M => {
          if off < last.1 {
            *last = (along, off);
          }
        }
        _ => found.push((along, off)),
      }
    }
    found.into_iter().map(|(along, _)| along).collect()
  }

  /// Closest to `near`, else the first
  fn locate(&self, p: &Point, near: f64) -> Option<f64> {
    self.candidates(p).into_iter().min_by(|a, b| (a - near).abs().total_cmp(&(b - near).abs()))
  }

  /// Next position ahead, same pass
  fn forward(&self, p: &Point, from: f64) -> Option<f64> {
    self.candidates(p).into_iter().filter(|&s| s >= from - BACK_M).min_by(|a, b| a.total_cmp(b))
  }

  /// Bbox of `[from, to]` in degrees, with margin
  fn bounds(&self, from: f64, to: f64) -> (f64, f64, f64, f64) {
    let margin = 300.0;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (&(x, y), &s) in self.xy.iter().zip(&self.cum) {
      if s >= from - margin && s <= to + margin {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
      }
    }
    let lat = |y: f64| self.lat0 + y / 110_540.0;
    let lng = |x: f64| self.lng0 + x / self.kx;
    (lat(y0 - margin), lng(x0 - margin), lat(y1 + margin), lng(x1 + margin))
  }

  /// Furthest position before `limit`
  fn before(&self, p: &Point, limit: f64) -> Option<f64> {
    self.candidates(p).into_iter().filter(|&s| s < limit).max_by(|a, b| a.total_cmp(b))
  }
}

static SHAPES: LazyLock<RwLock<HashMap<String, Arc<Polyline>>>> = LazyLock::new(Default::default);

/// Reloaded after a sync
pub(crate) fn forget_shapes(region: &str, provider: &str) {
  let prefix = format!("{region}:{provider}:");
  if let Ok(mut shapes) = SHAPES.write() {
    shapes.retain(|key, _| !key.starts_with(&prefix));
  }
}

/// Loaded once per sync
async fn polyline(db: &crate::Db, scope: &Scope, shape_id: &str) -> Option<Arc<Polyline>> {
  let key = format!("{}:{}:{shape_id}", scope.region.id(), scope.provider);
  if let Some(found) = SHAPES.read().ok()?.get(&key) {
    return Some(found.clone());
  }
  let json = Polyline::query_geojson(db, scope, shape_id).await?;
  let geojson: serde_json::Value = serde_json::from_str(&json).ok()?;
  let coords: Vec<(f64, f64)> = geojson["coordinates"]
    .as_array()?
    .iter()
    .filter_map(|c| Some((c.get(0)?.as_f64()?, c.get(1)?.as_f64()?)))
    .collect();
  let line = Arc::new(Polyline::new(&coords)?);
  SHAPES.write().ok()?.insert(key, line.clone());
  Some(line)
}

/// Recent travel time over `[from, to]` per piece, dwell included
fn observed_secs(line: &Polyline, trails: &[Vec<TrailPoint>], from: f64, to: f64, since: i64) -> Option<f64> {
  let bins = ((to - from) / BIN_M).ceil() as usize;
  if bins == 0 {
    return None;
  }
  // pace per bus and piece
  let mut paces: Vec<Vec<f64>> = vec![Vec::new(); bins];
  for trail in trails {
    let mut spent = vec![(0.0_f64, 0.0_f64); bins];
    let mut last: Option<(i64, f64)> = None;
    for point in trail.iter().filter(|p| p.t >= since) {
      let s = match last {
        Some((_, prev)) => line.forward(&point.at, prev),
        None => line.locate(&point.at, from),
      };
      let Some(s) = s else { continue };
      let Some((t0, s0)) = last else {
        last = Some((point.t, s));
        continue;
      };
      let (dt, ds) = (point.t - t0, s - s0);
      if ds <= 1.0 {
        continue;
      }
      if dt > 0 && dt <= MAX_GAP_S && ds / dt as f64 <= MAX_MPS {
        let pace = dt as f64 / ds;
        for (i, bin) in spent.iter_mut().enumerate() {
          let (lo, hi) = (from + i as f64 * BIN_M, (from + (i + 1) as f64 * BIN_M).min(to));
          let overlap = s.min(hi) - s0.max(lo);
          if overlap > 0.0 {
            *bin = (bin.0 + overlap * pace, bin.1 + overlap);
          }
        }
      }
      last = Some((point.t, s));
    }
    for (bin, (secs, meters)) in paces.iter_mut().zip(spent) {
      if meters >= BIN_M / 2.0 {
        bin.push(secs / meters);
      }
    }
  }

  let medians: Vec<Option<f64>> = paces
    .into_iter()
    .map(|mut p| {
      p.sort_by(|a, b| a.total_cmp(b));
      (p.len() >= MIN_BUSES).then(|| p[p.len() / 2])
    })
    .collect();
  let covered: Vec<f64> = medians.iter().flatten().copied().collect();
  if (covered.len() as f64) < bins as f64 * MIN_COVERAGE {
    return None;
  }
  // uncovered pieces at the covered pace
  let fill = covered.iter().sum::<f64>() / covered.len() as f64;
  Some(
    medians
      .iter()
      .enumerate()
      .map(|(i, pace)| {
        let length = (to - from - i as f64 * BIN_M).min(BIN_M);
        length * pace.unwrap_or(fill)
      })
      .sum(),
  )
}

/// s to the stop at the observed pace, any line
pub async fn eta_secs(state: &crate::Transit, scope: &Scope, shape_id: &str, bus: &Point, stop: &Point) -> Option<f64> {
  let route = polyline(&state.db, scope, shape_id).await?;
  let stop_s = route.locate(stop, 0.0)?;
  let bus_s = route.before(bus, stop_s)?;
  let since = chrono::Utc::now().timestamp() - WINDOW_S;
  let trails = state.cache.tracker().trails_within(scope.provider, since, route.bounds(bus_s, stop_s)).await;
  observed_secs(&route, &trails, bus_s, stop_s, since)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn straight() -> Polyline {
    // 1 km straight east
    let end = 1000.0 / (111_320.0 * (-34.9_f64).to_radians().cos());
    Polyline::new(&[(-56.0, -34.9), (-56.0 + end, -34.9)]).unwrap()
  }

  fn at(line: &Polyline, meters: f64) -> Point {
    Point { lat: line.lat0, lng: line.lng0 + meters / line.kx }
  }

  #[test]
  fn paces_the_segment_with_what_buses_just_did() {
    let line = straight();
    // 1000 m at 5 m/s
    let trail: Vec<TrailPoint> = (0..=10).map(|i| TrailPoint { t: i * 20, at: at(&line, i as f64 * 100.0) }).collect();
    let secs = observed_secs(&line, &[trail.clone(), trail], 0.0, 1000.0, 0).unwrap();
    assert!((secs - 200.0).abs() < 5.0, "{secs}");
  }

  #[test]
  fn dwell_counts_and_little_coverage_gives_up() {
    let line = straight();
    // 300 m, 60 s stopped halfway
    let trail = vec![
      TrailPoint { t: 0, at: at(&line, 0.0) },
      TrailPoint { t: 30, at: at(&line, 150.0) },
      TrailPoint { t: 90, at: at(&line, 150.0) },
      TrailPoint { t: 120, at: at(&line, 300.0) },
    ];
    let two = [trail.clone(), trail];
    let secs = observed_secs(&line, &two, 0.0, 300.0, 0).unwrap();
    assert!((secs - 120.0).abs() < 5.0, "{secs}");
    assert!(observed_secs(&line, &two, 0.0, 1000.0, 0).is_none());
    assert!(observed_secs(&line, &two[..1], 0.0, 300.0, 0).is_none());
  }
}
