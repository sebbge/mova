use crate::geo::Point;

use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct BusEntry {
  pub id: String,
  pub line: String,
  pub company: String,
  pub location: Point,
  pub speed: Option<f64>,

  pub origin: Option<String>,
  pub destination: Option<String>,
  pub timestamp: Option<DateTime<Utc>>,

  pub access: Option<String>,
  pub emissions: Option<String>,
  pub thermal_comfort: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpcomingArrival {
  pub bus: BusEntry,
  pub eta_s: Option<i64>,
  pub distance_m: Option<i64>,
  pub position: Option<i64>,
}
