use crate::fare::{Fare, Rider};
use mova_core::{Ref, geo::Point};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug)]
pub struct Query {
  pub from: Point,
  pub to: Point,
  pub at: NaiveDateTime,
  pub rider: Rider,
  pub mode: Mode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
  #[default]
  Transit,
  Walk,
}

#[derive(Debug, Serialize)]
pub struct Itinerary {
  pub departure: NaiveDateTime,
  pub arrival: NaiveDateTime,
  pub duration_s: i64,
  pub transfers: u32,
  pub walk_m: u32,
  pub fare: Fare,
  pub legs: Vec<Leg>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Leg {
  Walk {
    from: Place,
    to: Place,
    departure: NaiveDateTime,
    arrival: NaiveDateTime,
    duration_s: i64,
    distance_m: u32,
  },
  Bus {
    provider: String,
    line: String,
    headsign: Option<String>,
    route_id: Ref,
    shape_id: Option<String>,
    from: Place,
    to: Place,
    departure: NaiveDateTime,
    arrival: NaiveDateTime,
    stops: u32,
    interpolated: bool,
    path: Vec<[f64; 2]>,
  },
}

impl Leg {
  pub fn departure(&self) -> NaiveDateTime {
    match self {
      Self::Walk { departure, .. } | Self::Bus { departure, .. } => *departure,
    }
  }

  pub fn arrival(&self) -> NaiveDateTime {
    match self {
      Self::Walk { arrival, .. } | Self::Bus { arrival, .. } => *arrival,
    }
  }
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Place {
  Stop { id: Ref, name: String, lat: f64, lng: f64 },
  Point { lat: f64, lng: f64 },
}

impl From<Point> for Place {
  fn from(p: Point) -> Self {
    Self::Point { lat: p.lat, lng: p.lng }
  }
}
