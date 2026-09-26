use crate::geo::Point;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct BusEntry {
  pub timestamp: Option<DateTime<Utc>>,

  pub id: String,
  pub unit: String,
  pub line: String,
  pub company: String,
  pub location: Point,
  pub speed: Option<f64>,

  pub shape_id: Option<String>,
  pub origin: Option<String>,
  pub destination: Option<String>,
  pub delay_min: Option<i64>,
  pub stop: Option<StopRef>,
  pub next_stop: Option<StopRef>,

  pub access: Option<String>,
  pub emissions: Option<String>,
  pub thermal_comfort: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopRef {
  pub id: String,
  pub name: String,
}

impl BusEntry {
  pub fn identity(company: &str, unit: &str) -> String {
    format!("{company}:{unit}")
  }
}

#[derive(Clone, Debug)]
pub struct UpcomingArrival {
  pub bus: BusEntry,
  pub eta_s: Option<i64>,
  pub distance_m: Option<i64>,
  pub position: Option<i64>,
}
