use crate::fare::FareCatalog;
use mova_core::{Ref, geo::Point};

use chrono::NaiveDate;
use rustc_hash::FxHashMap;

mod build;

pub struct Stop {
  pub id: Ref,
  pub name: String,
  pub location: Point,
}

pub struct Trip {
  pub route: Ref,
  pub line: String,
  pub headsign: Option<String>,
  pub shape_id: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct Time {
  pub arrival: i32,
  pub departure: i32,
  pub interpolated: bool,
}

pub struct Pattern {
  pub stops: Vec<u32>,
  pub trips: Vec<u32>,
  times: Vec<Time>,
}

impl Pattern {
  pub fn time(&self, row: usize, pos: usize) -> Time {
    self.times[row * self.stops.len() + pos]
  }

  pub fn earliest(&self, pos: usize, ready: i32) -> Option<usize> {
    let (mut lo, mut hi) = (0, self.trips.len());
    while lo < hi {
      let mid = (lo + hi) / 2;
      if self.time(mid, pos).departure < ready {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    (lo < self.trips.len()).then_some(lo)
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Walking {
  pub meters: f64,
  pub secs: i32,
}

impl Walking {
  const DETOUR: f64 = 1.3;
  const SPEED_MPS: f64 = 1.2;

  pub fn between(from: Point, to: Point) -> Self {
    let meters = from.distance_to(&to) * Self::DETOUR;
    Self { meters, secs: (meters / Self::SPEED_MPS).ceil() as i32 }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Transfer {
  pub to: u32,
  pub secs: i32,
  pub meters: f64,
}

pub struct Timetable {
  pub date: NaiveDate,
  pub stops: Vec<Stop>,
  pub trips: Vec<Trip>,
  pub patterns: Vec<Pattern>,
  pub stop_patterns: Vec<Vec<(u32, u32)>>,
  pub transfers: Vec<Vec<Transfer>>,
  pub fares: FareCatalog,
  grid: Grid,
}

impl Timetable {
  pub const MIN_CHANGE_S: i32 = 60;

  pub fn near(&self, point: Point, radius_m: f64) -> Vec<(u32, f64)> {
    self
      .grid
      .around(point, radius_m)
      .filter_map(|s| {
        let d = point.distance_to(&self.stops[s as usize].location);
        (d <= radius_m).then_some((s, d))
      })
      .collect()
  }
}

struct Grid {
  cells: FxHashMap<(i32, i32), Vec<u32>>,
}

impl Grid {
  const CELL_DEG: f64 = 0.005;
  const METERS_PER_DEG: f64 = 111_320.0;

  fn new(points: impl Iterator<Item = Point>) -> Self {
    let mut cells: FxHashMap<(i32, i32), Vec<u32>> = FxHashMap::default();
    for (i, p) in points.enumerate() {
      cells.entry(Self::cell(p)).or_default().push(i as u32);
    }
    Self { cells }
  }

  fn cell(p: Point) -> (i32, i32) {
    ((p.lat / Self::CELL_DEG).floor() as i32, (p.lng / Self::CELL_DEG).floor() as i32)
  }

  fn around(&self, p: Point, radius_m: f64) -> impl Iterator<Item = u32> + '_ {
    let lat_cells = (radius_m / Self::METERS_PER_DEG / Self::CELL_DEG).ceil() as i32;
    let lng_scale = p.lat.to_radians().cos().max(0.1);
    let lng_cells = (radius_m / (Self::METERS_PER_DEG * lng_scale) / Self::CELL_DEG).ceil() as i32;
    let (cy, cx) = Self::cell(p);
    (cy - lat_cells..=cy + lat_cells)
      .flat_map(move |y| (cx - lng_cells..=cx + lng_cells).map(move |x| (y, x)))
      .filter_map(|key| self.cells.get(&key))
      .flatten()
      .copied()
  }
}
