// https://api.montevideo.gub.uy/apidocs/publictransport
#![allow(unused)]
use crate::core::geo::Point;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

// common bus fields
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmBus {
  pub bus_id: u32,

  // "company" in /buses/.../upcomingbuses, "companyName" in /buses
  #[serde(alias = "companyName")]
  pub company: String,

  #[serde(deserialize_with = "deser_line")]
  pub line: String,

  pub line_variant_id: u32,
  pub special: bool,

  #[serde(deserialize_with = "deser_point")]
  pub location: Point,

  pub origin: String,
  pub destination: String,
  pub subline: String,

  pub emissions: StmBusEmissions,
  pub thermal_confort: StmBusAC,
  pub access: StmBusAccess,
}

///# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmUpcomingBus {
  #[serde(flatten)]
  pub bus: StmBus,

  pub eta: Option<i64>, // s
  pub distance: Option<i64>,
  pub position: Option<i64>,
}

///# GET <MVD>/buses?lines={lines}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmLineBus {
  #[serde(flatten)]
  pub bus: StmBus,

  #[serde(deserialize_with = "deser_timestamp")]
  pub timestamp: Option<DateTime<Utc>>,
  pub speed: Option<f64>,
  pub e_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StmGeo {
  pub coordinates: Vec<f64>,

  #[serde(rename = "type")]
  pub geo_type: Option<String>,
}
fn point_from(geo: StmGeo) -> Result<Point, &'static str> {
  match geo.geo_type.as_deref() {
    Some("Point") if geo.coordinates.len() == 2 => Ok(Point { lat: geo.coordinates[1], lng: geo.coordinates[0] }),
    Some("Point") => Err("coordenadas inválidas, se espera [lng, lat]"),
    _ => Err("tipo de geometría no soportado"),
  }
}

//- bus info

// CO2 emissions
#[derive(Debug)]
pub enum StmBusEmissions {
  NoData,
  Details(String),
}
impl<'de> Deserialize<'de> for StmBusEmissions {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => StmBusEmissions::NoData,
      _ => StmBusEmissions::Details(raw),
    })
  }
}

impl StmBusEmissions {
  pub fn label(&self) -> Option<String> {
    match self {
      StmBusEmissions::NoData => None,
      StmBusEmissions::Details(s) => Some(crate::core::text::sentence_case(s)),
    }
  }
}

// AC unit
#[derive(Debug)]
pub enum StmBusAC {
  NoData,
  Details(String),
}
impl<'de> Deserialize<'de> for StmBusAC {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => StmBusAC::NoData,
      _ => StmBusAC::Details(raw),
    })
  }
}

impl StmBusAC {
  pub fn label(&self) -> Option<String> {
    match self {
      StmBusAC::NoData => None,
      StmBusAC::Details(s) => Some(crate::core::text::sentence_case(s)),
    }
  }
}

// accessibility
#[derive(Debug)]
pub enum StmBusAccess {
  NoData,
  Common,
  Details(String),
}
impl<'de> Deserialize<'de> for StmBusAccess {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => StmBusAccess::NoData,
      "común" | "comun" => StmBusAccess::Common,
      _ => StmBusAccess::Details(raw),
    })
  }
}

impl StmBusAccess {
  pub fn label(&self) -> Option<String> {
    match self {
      StmBusAccess::NoData => None,
      StmBusAccess::Common => None,
      StmBusAccess::Details(s) => Some(crate::core::text::sentence_case(s)),
    }
  }
}

// desers
fn deser_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error> {
  let opt = Option::<String>::deserialize(deserializer)?;

  Ok(opt.and_then(|s| {
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
      return Some(dt.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(&format!("{s}:00")).ok().map(|dt| dt.with_timezone(&Utc))
  }))
}

fn deser_line<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
  Ok(crate::names::line_name(&String::deserialize(d)?))
}

fn deser_point<'de, D: Deserializer<'de>>(d: D) -> Result<Point, D::Error> {
  use serde::de::Error;
  point_from(StmGeo::deserialize(d)?).map_err(D::Error::custom)
}
