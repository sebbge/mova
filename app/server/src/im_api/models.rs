// https://api.montevideo.gub.uy/apidocs/publictransport
#![allow(unused)]

use serde::{Deserialize, Deserializer, Serialize};

// common bus fields
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImBus {
  pub bus_id: u32,

  // "company" in /buses/.../upcomingbuses, "companyName" in /buses
  #[serde(alias = "companyName")]
  pub company: String,

  pub line: String,
  pub line_variant_id: u32,
  pub special: bool,

  pub location: Point,
  pub origin: String,
  pub destination: String,
  pub subline: String,

  pub emissions: ImBusEmissions,
  pub thermal_confort: ImBusThermalComfort,
  pub access: ImBusAccess,
}
impl ImBus {
  pub fn get_uid(&self) -> String {
    format!("{}:{}", self.line, self.bus_id)
  }
}

///# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImUpcomingBus {
  #[serde(flatten)]
  pub bus: ImBus,

  pub eta: Option<i64>, // s
  pub distance: Option<i64>,
  pub position: Option<i64>,
}

///# GET <MVD>/buses?lines={lines}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImLineBus {
  #[serde(flatten)]
  pub bus: ImBus,

  #[serde(deserialize_with = "deser_timestamp")]
  pub timestamp: Option<DateTime<Utc>>,
  pub speed: Option<f64>,
  pub e_type: Option<String>,
}
use chrono::{DateTime, Utc};
fn deser_timestamp<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error> {
  let opt = Option::<String>::deserialize(deserializer)?;

  Ok(opt.and_then(|s| {
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
      return Some(dt.with_timezone(&Utc));
    }
    DateTime::parse_from_rfc3339(&format!("{s}:00")).ok().map(|dt| dt.with_timezone(&Utc))
  }))
}

#[derive(Clone, Debug, Serialize)]
pub struct Point {
  pub lat: f64,
  pub lng: f64,
}

impl<'de> serde::Deserialize<'de> for Point {
  fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    use serde::de::Error;

    // [lng, lat] wire or {lat, lng} cached
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Repr {
      Geo(ImGeo),
      LatLng { lat: f64, lng: f64 },
    }

    match Repr::deserialize(deserializer)? {
      Repr::Geo(geo) => Point::try_from(geo).map_err(D::Error::custom),
      Repr::LatLng { lat, lng } => Ok(Point { lat, lng }),
    }
  }
}
impl Point {
  pub fn get_distance_to(&self, other: &Point) -> f64 {
    // haversine
    const R: f64 = 6_371_000.0;
    let dlat = (other.lat - self.lat).to_radians();
    let dlng = (other.lng - self.lng).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
      + self.lat.to_radians().cos() * other.lat.to_radians().cos() * (dlng / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
  }

  pub fn get_bearing_to(&self, other: &Point) -> f64 {
    let (rlat1, rlat2) = (self.lat.to_radians(), other.lat.to_radians());
    let dlng = (other.lng - self.lng).to_radians();
    let y = dlng.sin() * rlat2.cos();
    let x = rlat1.cos() * rlat2.sin() - rlat1.sin() * rlat2.cos() * dlng.cos();
    (y.atan2(x).to_degrees() + 360.0) % 360.0
  }
}

#[derive(Debug, Deserialize)]
pub struct ImGeo {
  pub coordinates: Vec<f64>,

  #[serde(rename = "type")]
  pub geo_type: Option<String>,
}
impl TryFrom<ImGeo> for Point {
  type Error = &'static str;

  fn try_from(value: ImGeo) -> Result<Self, Self::Error> {
    match value.geo_type.as_deref() {
      Some("Point") => {
        if value.coordinates.len() != 2 {
          return Err("Invalid coordinate format, must be [lng, lat]");
        }
        Ok(Point { lat: value.coordinates[1], lng: value.coordinates[0] })
      }
      _ => Err("Geo type not supported"),
    }
  }
}
// bus info

// CO2 emissions
#[derive(Debug)]
pub enum ImBusEmissions {
  NoData,
  Details(String),
}
impl<'de> Deserialize<'de> for ImBusEmissions {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => ImBusEmissions::NoData,
      _ => ImBusEmissions::Details(raw),
    })
  }
}

impl ImBusEmissions {
  pub fn label(&self) -> Option<String> {
    match self {
      ImBusEmissions::NoData => None,
      ImBusEmissions::Details(s) => Some(crate::helpers::sentence_case(s)),
    }
  }
}

// AC unit
#[derive(Debug)]
pub enum ImBusThermalComfort {
  NoData,
  Details(String),
}
impl<'de> Deserialize<'de> for ImBusThermalComfort {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => ImBusThermalComfort::NoData,
      _ => ImBusThermalComfort::Details(raw),
    })
  }
}

impl ImBusThermalComfort {
  pub fn label(&self) -> Option<String> {
    match self {
      ImBusThermalComfort::NoData => None,
      ImBusThermalComfort::Details(s) => Some(crate::helpers::sentence_case(s)),
    }
  }
}

// accessibility
#[derive(Debug)]
pub enum ImBusAccess {
  NoData,
  Common,
  Details(String),
}
impl<'de> Deserialize<'de> for ImBusAccess {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    let raw = String::deserialize(deserializer)?;
    Ok(match raw.to_lowercase().as_str() {
      "sin datos" => ImBusAccess::NoData,
      "común" | "comun" => ImBusAccess::Common,
      _ => ImBusAccess::Details(raw),
    })
  }
}

impl ImBusAccess {
  pub fn label(&self) -> Option<String> {
    match self {
      ImBusAccess::NoData => None,
      ImBusAccess::Common => None,
      ImBusAccess::Details(s) => Some(crate::helpers::sentence_case(s)),
    }
  }
}
