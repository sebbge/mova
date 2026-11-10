// https://api.montevideo.gub.uy/apidocs/publictransport
use crate::sdk::format;
use crate::sdk::geo::Point;
use crate::sdk::{arrivals::UpcomingArrival, vehicles::BusEntry};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

//# GET <MVD>/buses/linevariants
// `lineVariantId` is the GTFS `shape_id`
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmLineVariant {
  pub line_variant_id: u32,
  pub destination: Option<String>,
}

// common bus fields
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmBus {
  pub bus_id: u32,

  #[serde(alias = "companyName", default, deserialize_with = "deser_nullable")]
  pub company: String,

  #[serde(default, deserialize_with = "deser_line")]
  pub line: String,

  #[serde(deserialize_with = "deser_point")]
  pub location: Point,

  pub line_variant_id: Option<u32>,
  pub origin: Option<String>,
  pub destination: Option<String>,

  #[serde(default, deserialize_with = "deser_label")]
  pub emissions: Option<String>, // CO2 emissions

  #[serde(default, deserialize_with = "deser_label")]
  #[serde(rename = "thermalConfort")]
  pub thermal_comfort: Option<String>, // AC unit
  #[serde(default, deserialize_with = "deser_label")]
  pub access: Option<String>, // accessibility
}

impl From<StmBus> for BusEntry {
  fn from(bus: StmBus) -> Self {
    BusEntry {
      id: BusEntry::identity(&bus.company, &bus.bus_id.to_string()),
      unit: bus.bus_id.to_string(),
      line: bus.line,
      company: bus.company,
      location: bus.location,
      speed: None,
      shape_id: bus.line_variant_id.map(|v| v.to_string()),
      origin: bus.origin.as_deref().map(format::place_name),
      destination: bus.destination.as_deref().map(format::place_name),
      timestamp: None,
      access: bus.access,
      emissions: bus.emissions,
      thermal_comfort: bus.thermal_comfort,
      delay_min: None,
      stop: None,
      next_stop: None,
    }
  }
}

///# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StmUpcomingBus {
  #[serde(flatten)]
  pub bus: StmBus,

  pub eta: Option<i64>, // s
  pub distance: Option<i64>,
  pub position: Option<i64>,
}

impl From<StmUpcomingBus> for UpcomingArrival {
  fn from(im: StmUpcomingBus) -> Self {
    UpcomingArrival { eta_s: im.eta, distance_m: im.distance, position: im.position, bus: im.bus.into() }
  }
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
}

impl From<StmLineBus> for BusEntry {
  fn from(im: StmLineBus) -> Self {
    BusEntry { speed: im.speed, timestamp: im.timestamp, ..im.bus.into() }
  }
}

#[derive(Debug, Deserialize)]
struct StmGeo {
  coordinates: Vec<f64>,

  #[serde(rename = "type")]
  geo_type: Option<String>,
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
  Ok(Option::<String>::deserialize(d)?.map(|raw| format::line_name(&raw)).unwrap_or_default())
}

fn deser_label<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
  let raw = Option::<String>::deserialize(d)?;
  Ok(
    raw
      .filter(|s| !matches!(s.trim().to_lowercase().as_str(), "" | "sin datos" | "común" | "comun"))
      .map(|s| format::sentence_case(&s)),
  )
}

fn deser_nullable<'de, D: Deserializer<'de>, T: Deserialize<'de> + Default>(d: D) -> Result<T, D::Error> {
  Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn deser_point<'de, D: Deserializer<'de>>(d: D) -> Result<Point, D::Error> {
  use serde::de::Error;

  let geo = StmGeo::deserialize(d)?;
  match (geo.geo_type.as_deref(), geo.coordinates.as_slice()) {
    (Some("Point"), &[lng, lat]) => Ok(Point { lat, lng }),
    (Some("Point"), _) => Err(D::Error::custom("invalid coordinates, expected [lng, lat]")),
    _ => Err(D::Error::custom("unsupported geometry type")),
  }
}
