// https://seguitubus.trescruces.com.uy SeguíTuBus, Tres Cruces
use super::index::Index;
use crate::core::provider::prelude::*;

use chrono::{Datelike, FixedOffset, NaiveDateTime, Utc};
use serde::Deserialize;

const URL: &str = "https://seguitubus.trescruces.com.uy/api/auth/app/WSlistservmap";
const FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const UTC_OFFSET_S: i32 = -3 * 3600;

#[derive(Deserialize)]
struct Envelope {
  response: Listing,
}

#[derive(Deserialize)]
struct Listing {
  #[serde(default)]
  #[serde(rename = "servicios")]
  services: Vec<Service>,
  #[serde(default)]
  pass: i64,
}

#[derive(Deserialize)]
pub struct Service {
  #[serde(rename = "NumBus")]
  bus_number: String,
  #[serde(rename = "Empresa")]
  company: String,
  #[serde(rename = "Origen")]
  origin: String,
  #[serde(rename = "Destino")]
  destination: String,
  #[serde(rename = "HoraInicioTeorica")]
  start: String,
  #[serde(rename = "HoraPartidaProg", default)]
  scheduled_departure: Option<String>,
  #[serde(rename = "HoraPartidaReal", default)]
  actual_departure: Option<String>,
  #[serde(rename = "fechaHora", default)]
  reported_at: Option<String>,
  #[serde(default)]
  latitude: Option<f64>,
  #[serde(default)]
  longitude: Option<f64>,
  #[serde(rename = "GPS", default)]
  gps: i64,
}

//# POST <STB>/WSlistservmap
pub async fn services(client: &reqwest::Client) -> Result<Vec<Service>> {
  let req = client.post(URL).header("X-Requested-With", "XMLHttpRequest").json(&serde_json::json!({ "filtros": {} }));
  let res = http::fetch(req, "[inter/seguitubus]").await?;
  let Envelope { response } = http::json(res, "[inter/seguitubus]").await?;

  if response.pass != 1 {
    tracing::warn!(pass = response.pass, "[inter/seguitubus] request rejected");
    return Err(Error::External("SeguiTuBus rejected the request".into()));
  }
  if response.services.is_empty() {
    tracing::warn!("[inter/seguitubus] no active services");
    return Err(Error::External("SeguiTuBus returned no active services".into()));
  }
  Ok(response.services)
}

impl Service {
  pub fn into_bus(self, index: &Index) -> Option<BusEntry> {
    let (Some(lat), Some(lng)) = (self.latitude, self.longitude) else {
      return None;
    };
    if self.gps != 1 || lat == 0.0 || lng == 0.0 {
      return None;
    }
    let line = index.line(&self.company, &self.origin, &self.destination, local(&self.start)?)?;

    let delay_min =
      match (self.scheduled_departure.as_deref().and_then(local), self.actual_departure.as_deref().and_then(local)) {
        (Some(scheduled), Some(actual)) => Some((actual - scheduled).num_minutes()),
        _ => None,
      };
    let timestamp = self.reported_at.as_deref().and_then(local).and_then(|t| {
      let offset = FixedOffset::east_opt(UTC_OFFSET_S)?;
      Some(t.and_local_timezone(offset).single()?.with_timezone(&Utc))
    });

    Some(BusEntry {
      id: BusEntry::identity(&self.company, &self.bus_number),
      unit: self.bus_number,
      line: format::line_name(line),
      company: self.company,
      location: Point { lat, lng },
      speed: None,
      shape_id: None,
      origin: Some(format::place_name(&self.origin)),
      destination: Some(format::place_name(&self.destination)),
      timestamp,
      delay_min,
      stop: None,
      next_stop: None,
      access: None,
      emissions: None,
      thermal_comfort: None,
    })
  }
}

fn local(raw: &str) -> Option<NaiveDateTime> {
  NaiveDateTime::parse_from_str(raw.trim(), FORMAT).ok().filter(|t| t.year() > 1)
}
