use crate::api::csv::fields;
use crate::names::match_key;
use crate::sdk::prelude::*;

use std::collections::HashMap;

pub const URL: &str = "https://docs.google.com/spreadsheets/d/1kNokAyl1U_b_vyaMsN4eAK3bYXZgWMvxCdaSp6IW2hc/gviz/tq?tqx=out:csv&sheet=BASE_HORARIOS";
const COLUMNS: [&str; 9] =
  ["Servicio", "Línea", "SubLínea", "Sentido", "Orden_Parada", "Parada", "Hora", "Referencia", "Activo"];

pub struct Trip {
  pub service: String,
  pub line: String,
  pub subline: String,
  pub inbound: bool,
  pub days: Days,
  pub passings: Vec<Passing>,
}

pub struct Passing {
  pub stop: String,
  pub minute: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Days {
  Weekdays,
  MonToSat,
  Daily,
}

impl Days {
  fn from_reference(raw: &str) -> Self {
    let raw = raw.to_uppercase();
    if raw.contains('S') {
      Self::Weekdays
    } else if raw.contains('D') {
      Self::MonToSat
    } else {
      Self::Daily
    }
  }

  pub fn service_id(self) -> &'static str {
    match self {
      Self::Weekdays => "LV",
      Self::MonToSat => "LS",
      Self::Daily => "LD",
    }
  }

  pub fn week(self) -> [bool; 7] {
    match self {
      Self::Weekdays => [true, true, true, true, true, false, false],
      Self::MonToSat => [true, true, true, true, true, true, false],
      Self::Daily => [true; 7],
    }
  }
}

struct Row {
  order: u32,
  stop: String,
  minute: Option<u32>,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Trip>> {
  let text = String::from_utf8_lossy(bytes);
  let mut lines = text.lines();

  let header: Vec<String> = fields(lines.next().unwrap_or_default()).iter().map(|h| match_key(h)).collect();
  let col = |name: &str| header.iter().position(|h| *h == match_key(name));
  let Some(idx) = COLUMNS.iter().map(|c| col(c)).collect::<Option<Vec<_>>>() else {
    return Err(Error::External("CODESA sheet is missing columns".into()));
  };
  let [service, line, subline, direction, order, stop, hour, reference, active] = idx[..] else {
    return Err(Error::External("CODESA sheet is missing columns".into()));
  };

  let mut trips: Vec<Trip> = Vec::new();
  let mut rows: HashMap<String, Vec<Row>> = HashMap::new();
  let mut inactive = 0usize;

  for record in lines {
    let cols = fields(record);
    let get = |i: usize| cols.get(i).map(|s| s.trim()).unwrap_or_default();
    let id = get(service);
    if id.is_empty() {
      continue;
    }

    if !rows.contains_key(id) {
      if !get(active).eq_ignore_ascii_case("si") {
        inactive += 1;
        rows.insert(id.to_string(), Vec::new());
        continue;
      }
      trips.push(Trip {
        service: id.to_string(),
        line: get(line).to_string(),
        subline: get(subline).to_string(),
        inbound: get(direction).eq_ignore_ascii_case("vuelta"),
        days: Days::from_reference(get(reference)),
        passings: Vec::new(),
      });
    }
    rows.entry(id.to_string()).or_default().push(Row {
      order: get(order).parse().unwrap_or(u32::MAX),
      stop: get(stop).to_string(),
      minute: minute(get(hour)),
    });
  }

  for trip in &mut trips {
    let mut own = rows.remove(&trip.service).unwrap_or_default();
    own.sort_by_key(|f| f.order);
    trip.passings = own.into_iter().filter_map(|f| Some(Passing { minute: f.minute?, stop: f.stop })).collect();
  }

  if trips.is_empty() {
    return Err(Error::External("CODESA sheet came out empty".into()));
  }
  tracing::debug!(trips = trips.len(), inactive = inactive, "[codesa/sheet] parsed");
  Ok(trips)
}

fn minute(raw: &str) -> Option<u32> {
  let (h, m) = raw.split_once(':')?;
  let (h, m): (u32, u32) = (h.trim().parse().ok()?, m.trim().get(..2)?.parse().ok()?);
  (m < 60).then_some(h * 60 + m)
}
