use crate::sdk::prelude::*;

use serde::Deserialize;

const URL: &str = "https://www.montevideo.gub.uy/buses/rest/stm-online";
const SUBSISTEMAS: [i32; 2] = [3, 4];

#[derive(Deserialize)]
struct Collection {
  #[serde(default)]
  features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
  properties: Properties,
  geometry: Geometry,
}

#[derive(Deserialize)]
struct Geometry {
  #[serde(default)]
  coordinates: Vec<f64>,
}

#[derive(Deserialize)]
struct Properties {
  #[serde(rename = "codigoBus")]
  bus_code: i64,
  #[serde(rename = "codigoEmpresa")]
  company_code: i64,
  #[serde(rename = "linea")]
  line: String,
  #[serde(rename = "variante")]
  variant: i64,
  #[serde(rename = "destinoDesc")]
  destination: Option<String>,
  subsistema: i32,
  #[serde(rename = "velocidad")]
  speed: Option<f64>,
}

pub struct OnlineBus {
  pub bus_code: i64,
  pub company_code: i64,
  pub line: String,
  pub variant: i64,
  pub destination: Option<String>,
  pub speed: Option<f64>,
  pub location: Point,
}

impl From<OnlineBus> for BusEntry {
  fn from(bus: OnlineBus) -> Self {
    BusEntry {
      id: BusEntry::identity(&bus.company_code.to_string(), &bus.bus_code.to_string()),
      unit: bus.bus_code.to_string(),
      line: format::line_name(&bus.line),
      company: crate::names::company_name(bus.company_code),
      location: bus.location,
      speed: bus.speed,
      shape_id: Some(bus.variant.to_string()),
      origin: None,
      destination: bus.destination.as_deref().map(format::place_name),
      timestamp: None,
      access: None,
      emissions: None,
      thermal_comfort: None,
      delay_min: None,
      stop: None,
      next_stop: None,
    }
  }
}

pub async fn buses(client: &reqwest::Client) -> Result<Vec<OnlineBus>> {
  let [first, second] = SUBSISTEMAS;
  let (mut buses, rest) = tokio::try_join!(subsystem(client, first), subsystem(client, second))?;
  buses.extend(rest);
  Ok(buses)
}

async fn subsystem(client: &reqwest::Client, subsistema: i32) -> Result<Vec<OnlineBus>> {
  let body = serde_json::json!({ "subsistema": subsistema });
  let res = http::fetch(client.post(URL).json(&body), "[dnt/online]").await?;
  let collection: Collection = http::json(res, "[dnt/online]").await?;

  Ok(collection.features.into_iter().filter_map(Feature::into_bus).collect())
}

impl Feature {
  fn into_bus(self) -> Option<OnlineBus> {
    let Feature { properties: p, geometry } = self;

    if !SUBSISTEMAS.contains(&p.subsistema) {
      return None;
    }
    let [lng, lat] = geometry.coordinates[..] else {
      return None;
    };

    Some(OnlineBus {
      bus_code: p.bus_code,
      company_code: p.company_code,
      line: p.line,
      variant: p.variant,
      destination: p.destination,
      speed: p.speed,
      location: Point { lat, lng },
    })
  }
}
