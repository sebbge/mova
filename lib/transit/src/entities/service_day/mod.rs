use crate::sdk::{Ref, geo::Point};

use chrono::NaiveDate;

mod repository;

pub struct ServiceDay {
  pub date: NaiveDate,
  pub stops: Vec<DayStop>,
  pub trips: Vec<DayTrip>,
}

pub struct DayStop {
  pub id: Ref,
  pub name: String,
  pub location: Point,
  pub cluster: String,
}

pub struct DayTrip {
  pub route: Ref,
  pub line: String,
  pub headsign: Option<String>,
  pub shape_id: Option<String>,
  pub stop_times: Vec<DayStopTime>,
}

/// s from midnight, may be negative or past 24 h
#[derive(Clone, Copy, Debug)]
pub struct DayStopTime {
  pub stop: u32,
  pub arrival: i32,
  pub departure: i32,
  pub interpolated: bool,
}
