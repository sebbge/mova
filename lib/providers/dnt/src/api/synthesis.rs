use super::feed::write_entry;
use super::long_distance::{Corridor, Trip};
use super::places::Place;
use crate::sdk::{Error, Result};

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::Cursor;

const AGENCY_URL: &str = "https://www.gub.uy/ministerio-transporte-obras-publicas";
const DAYS: [&str; 7] = ["Lu", "Ma", "Mi", "Ju", "Vi", "Sa", "Do"];

pub fn gtfs(
  trips: &[Trip],
  corridors: &HashMap<(String, String), Corridor>,
  places: &HashMap<&str, Place>,
) -> Result<Vec<u8>> {
  let mut agency = BTreeMap::new();
  let mut routes = BTreeMap::new();
  let mut services = BTreeMap::new();
  let mut used = BTreeMap::new();
  let (mut trip_lines, mut stop_times) = (String::new(), String::new());
  let (mut holiday_only, mut unplaced, mut too_short) = (0usize, 0usize, 0usize);

  for t in trips {
    let Some(corridor) = corridors.get(&(t.company_code.clone(), t.line_code.clone())) else {
      continue;
    };
    let week = DAYS.map(|d| t.days.iter().any(|x| x == d));
    if !week.contains(&true) {
      holiday_only += 1;
      continue;
    }

    let placed: Vec<_> = t.passings.iter().filter(|p| places.contains_key(p.place.as_str())).collect();
    unplaced += t.passings.len() - placed.len();
    if placed.len() < 2 {
      too_short += 1;
      continue;
    }

    let service_id = format!(
      "{}-{}-{}",
      DAYS.iter().zip(week).filter(|(_, on)| *on).map(|(d, _)| *d).collect::<String>(),
      compact(&t.start),
      compact(&t.end)
    );
    services.entry(service_id.clone()).or_insert((week, compact(&t.start), compact(&t.end)));
    agency.entry(t.company_code.clone()).or_insert_with(|| t.company.clone());
    routes.entry(corridor.route_id.clone()).or_insert((t.company_code.clone(), corridor));

    let _ = writeln!(
      trip_lines,
      "{},{},{},{},{}",
      corridor.route_id,
      service_id,
      t.id,
      corridor.direction_id,
      quote(&t.destination)
    );
    for (seq, p) in placed.iter().enumerate() {
      let time = clock(p.minute);
      let _ = writeln!(stop_times, "{},{time},{time},{},{seq}", t.id, p.place);
      used.entry(p.place.as_str()).or_insert(());
    }
  }

  if holiday_only + unplaced + too_short > 0 {
    tracing::warn!(holiday_only, unplaced, too_short, "[inter/feed] trips or stops left out");
  }
  if routes.is_empty() {
    return Err(Error::External("long-distance feed came out empty".into()));
  }

  let mut files: Vec<(&str, String)> = Vec::new();

  let mut txt = String::from("agency_id,agency_name,agency_url,agency_timezone\n");
  for (id, name) in &agency {
    let _ = writeln!(txt, "{id},{},{AGENCY_URL},America/Montevideo", quote(name));
  }
  files.push(("agency.txt", txt));

  let mut txt = String::from("stop_id,stop_name,stop_lat,stop_lon\n");
  for code in used.keys() {
    if let Some(l) = places.get(code) {
      let _ = writeln!(txt, "{code},{},{},{}", quote(l.name), l.location.lat, l.location.lng);
    }
  }
  files.push(("stops.txt", txt));

  let mut txt = String::from("route_id,agency_id,route_short_name,route_long_name,route_type\n");
  for (id, (agency_id, c)) in &routes {
    let _ = writeln!(txt, "{id},{agency_id},{},{},3", quote(&c.short_name), quote(&c.long_name));
  }
  files.push(("routes.txt", txt));

  let mut txt =
    String::from("service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n");
  for (id, (week, start, end)) in &services {
    let days = week.map(|on| u8::from(on).to_string()).join(",");
    let _ = writeln!(txt, "{id},{days},{start},{end}");
  }
  files.push(("calendar.txt", txt));

  files.push(("trips.txt", format!("route_id,service_id,trip_id,direction_id,trip_headsign\n{trip_lines}")));
  files.push(("stop_times.txt", format!("trip_id,arrival_time,departure_time,stop_id,stop_sequence\n{stop_times}")));

  let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
  let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
  for (name, body) in files {
    write_entry(&mut out, name, body.as_bytes(), options)?;
  }
  let cursor = out.finish().map_err(|e| Error::External(format!("couldn't close zip: {e}")))?;
  Ok(cursor.into_inner())
}

fn clock(minutes: u32) -> String {
  format!("{:02}:{:02}:00", minutes / 60, minutes % 60)
}

fn compact(date: &str) -> String {
  date.replace('-', "")
}

fn quote(s: &str) -> String {
  format!("\"{}\"", s.replace('"', "\"\""))
}
