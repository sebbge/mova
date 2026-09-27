use crate::sdk::prelude::*;

use serde::Deserialize;

//# POST <IM>/buses/rest/stm-online
const URL: &str = "https://www.montevideo.gub.uy/buses/rest/stm-online";
const SUBSYSTEM: i32 = 1;

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
  bus_code: u32,
  #[serde(rename = "codigoEmpresa")]
  company_code: u32,
  #[serde(rename = "linea")]
  line: String,
  #[serde(rename = "variante")]
  variant: Option<u32>,
  #[serde(rename = "destinoDesc")]
  destination: Option<String>,
  #[serde(rename = "velocidad")]
  speed: Option<f64>,
}

pub struct OnlineBus {
  pub bus_code: u32,
  pub company_code: u32,
  pub line: String,
  pub variant: Option<u32>,
  pub destination: Option<String>,
  pub speed: Option<f64>,
  pub location: Point,
}

impl From<OnlineBus> for BusEntry {
  fn from(bus: OnlineBus) -> Self {
    let company = crate::names::company_name(bus.company_code);
    let unit = bus.bus_code.to_string();
    BusEntry {
      id: BusEntry::identity(&company, &unit),
      unit,
      line: format::line_name(&bus.line),
      company,
      location: bus.location,
      speed: bus.speed,
      shape_id: bus.variant.map(|v| v.to_string()),
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
  let body = serde_json::json!({ "subsistema": SUBSYSTEM });
  let res = http::fetch(client.post(URL).json(&body), "[stm/online]").await?;
  let collection: Collection = http::json(res, "[stm/online]").await?;

  let buses: Vec<OnlineBus> = collection.features.into_iter().filter_map(Feature::into_bus).collect();
  if buses.is_empty() {
    return Err(Error::External("stm-online returned no buses".into()));
  }
  tracing::info!(buses = buses.len(), "[stm/online] OK");
  Ok(buses)
}

impl Feature {
  fn into_bus(self) -> Option<OnlineBus> {
    let Feature { properties: p, geometry } = self;
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
