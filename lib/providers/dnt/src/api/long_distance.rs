// horarios_largadistancia_dnt.csv
use crate::sdk::prelude::*;

use std::collections::HashMap;

const DAY: u32 = 24 * 60;
const COLUMNS: [&str; 13] = [
  "Id.Turno",
  "Cod_Empresa",
  "Empresa",
  "Cod_Linea",
  "Cod_Origen",
  "Origen",
  "Cod_Destino",
  "Destino",
  "Dias",
  "F.Inicio",
  "F.Fin",
  "H.Salida",
  "Cod_Lugar",
];

pub struct Trip {
  pub id: String,
  pub company_code: String,
  pub company: String,
  pub line_code: String,
  pub origin_code: String,
  pub origin: String,
  pub destination_code: String,
  pub destination: String,
  pub days: Vec<String>,
  pub start: String,
  pub end: String,
  pub departure: u32,
  pub passings: Vec<Passing>,
}

pub struct Passing {
  pub place: String,
  pub minute: u32,
}

pub struct Corridor {
  pub route_id: String,
  pub short_name: String,
  pub long_name: String,
  pub direction_id: u8,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Trip>> {
  let text: String = bytes.iter().map(|&b| b as char).collect();
  let mut lines = text.lines();

  let header: Vec<&str> = lines.next().unwrap_or_default().split(';').map(str::trim).collect();
  let col = |name: &str| header.iter().position(|h| *h == name);
  let Some(idx) = COLUMNS.iter().map(|c| col(c)).collect::<Option<Vec<_>>>() else {
    return Err(Error::External("long-distance CSV is missing columns".into()));
  };
  let hour = col("Hora").ok_or_else(|| Error::External("long-distance CSV has no Hora".into()))?;

  let mut trips: Vec<Trip> = Vec::new();
  let mut by_id: HashMap<(String, String), usize> = HashMap::new();
  let mut dropped = 0usize;

  for line in lines.filter(|l| !l.trim().is_empty()) {
    let cols: Vec<&str> = line.split(';').map(str::trim).collect();
    let get = |i: usize| cols.get(idx[i]).copied().unwrap_or_default();
    let (Some(departure), Some(passed)) = (minutes(get(11)), cols.get(hour).and_then(|h| minutes(h))) else {
      dropped += 1;
      continue;
    };

    let i = *by_id.entry((get(1).to_string(), get(0).to_string())).or_insert_with(|| {
      trips.push(Trip {
        id: format!("{}.{}", get(1), get(0)),
        company_code: get(1).to_string(),
        company: get(2).to_string(),
        line_code: get(3).to_string(),
        origin_code: get(4).to_string(),
        origin: get(5).to_string(),
        destination_code: get(6).to_string(),
        destination: get(7).to_string(),
        days: get(8).split(',').map(str::trim).filter(|d| !d.is_empty()).map(String::from).collect(),
        start: get(9).to_string(),
        end: get(10).to_string(),
        departure,
        passings: Vec::new(),
      });
      trips.len() - 1
    });

    let trip = &mut trips[i];
    let minute = trip.departure + (passed + DAY - trip.departure) % DAY;
    trip.passings.push(Passing { place: get(12).to_string(), minute });
  }

  if dropped > 0 {
    tracing::warn!(dropped, "[inter/csv] rows without a valid time");
  }
  if trips.is_empty() {
    return Err(Error::External("long-distance CSV has no trips".into()));
  }

  for trip in &mut trips {
    trip.passings.sort_by_key(|p| p.minute);
  }
  Ok(trips)
}

pub fn corridors(trips: &[Trip]) -> HashMap<(String, String), Corridor> {
  type End<'a> = (&'a str, &'a str);
  let mut groups: HashMap<(&str, End), Vec<&Trip>> = HashMap::new();
  for t in trips {
    let pair = if t.origin_code <= t.destination_code {
      (&*t.origin_code, &*t.destination_code)
    } else {
      (&*t.destination_code, &*t.origin_code)
    };
    groups.entry((&t.company_code, pair)).or_default().push(t);
  }

  let mut out = HashMap::new();
  for members in groups.values() {
    let Some(base) = members.iter().min_by_key(|t| (t.line_code.parse::<u32>().unwrap_or(u32::MAX), &t.line_code))
    else {
      continue;
    };
    for t in members {
      let direction_id = u8::from(t.origin_code != base.origin_code);
      out.entry((t.company_code.clone(), t.line_code.clone())).or_insert_with(|| Corridor {
        route_id: format!("{}.{}", base.company_code, base.line_code),
        short_name: format!("{} {}", base.company, base.line_code),
        long_name: format!("{} - {}", base.origin, base.destination),
        direction_id,
      });
    }
  }
  out
}

pub fn minutes(raw: &str) -> Option<u32> {
  let (h, m) = raw.trim().split_once(':')?;
  let (h, m): (u32, u32) = (h.parse().ok()?, m.get(..2).unwrap_or(m).parse().ok()?);
  (h < 48 && m < 60).then_some(h * 60 + m)
}
