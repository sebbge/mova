use super::kmz::Stop;
use super::points;
use super::sheet::{Days, Trip};
use crate::names::match_key;
use crate::sdk::prelude::*;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::io::{Cursor, Write as _};

const AGENCY: &str = "codesa,CODESA,https://www.codesa.com.uy,America/Montevideo";
const VALIDITY: (&str, &str) = ("20260101", "20991231");

pub struct Variant {
  pub stops: Vec<Stop>,
  pub shape: Option<Vec<(f64, f64)>>,
}

pub type Variants = HashMap<(String, String), Variant>;

pub fn gtfs(trips: &[Trip], variants: &Variants) -> Result<Vec<u8>> {
  let table = points::table();
  let mut stops: BTreeMap<&str, &Stop> = BTreeMap::new();
  let mut shapes: BTreeSet<(&str, &str)> = BTreeSet::new();
  let mut services: BTreeSet<Days> = BTreeSet::new();
  let mut routes: BTreeMap<&str, HashMap<(&str, bool), usize>> = BTreeMap::new();
  let (mut trip_lines, mut stop_times) = (String::new(), String::new());
  let (mut without_variant, mut unanchored) = (0usize, 0usize);

  for v in trips {
    let Some(variant) = variants.get(&(v.line.clone(), v.subline.clone())) else {
      without_variant += 1;
      continue;
    };
    let anchors = anchor(v, &variant.stops, &table);
    if anchors.len() < 2 {
      unanchored += 1;
      continue;
    }

    let shape_id = match variant.shape {
      Some(_) => {
        shapes.insert((&v.line, &v.subline));
        format!("{}_{}", v.line, v.subline)
      }
      None => String::new(),
    };
    let headsign = anchors.last().map_or("", |&(i, _)| variant.stops[i].name.as_str());
    let _ = writeln!(
      trip_lines,
      "{},{},{},{},{shape_id},{}",
      v.line,
      v.days.service_id(),
      v.service,
      u8::from(v.inbound),
      quote(headsign)
    );

    for (i, seconds) in times(&variant.stops, &anchors) {
      let stop = &variant.stops[i];
      let time = clock(seconds);
      let _ = writeln!(stop_times, "{},{time},{time},{},{i}", v.service, stop.code);
      stops.entry(&stop.code).or_insert(stop);
    }
    services.insert(v.days);
    *routes.entry(&v.line).or_default().entry((&v.subline, v.inbound)).or_default() += 1;
  }

  if without_variant + unanchored > 0 {
    tracing::warn!(without_stops = without_variant, unanchored = unanchored, "[codesa/feed] trips left out");
  }
  if routes.is_empty() {
    return Err(Error::External("CODESA feed came out empty".into()));
  }

  let mut files: Vec<(&str, String)> =
    vec![("agency.txt", format!("agency_id,agency_name,agency_url,agency_timezone\n{AGENCY}\n"))];

  let mut txt = String::from("stop_id,stop_name,stop_lat,stop_lon\n");
  for (code, p) in &stops {
    let _ = writeln!(txt, "{code},{},{},{}", quote(&p.name), p.location.lat, p.location.lng);
  }
  files.push(("stops.txt", txt));

  let mut txt = String::from("route_id,agency_id,route_short_name,route_long_name,route_type\n");
  for (line, usage) in &routes {
    let main = usage.iter().max_by_key(|((sub, inbound), n)| (!inbound, **n, std::cmp::Reverse(*sub))).map(|(k, _)| k);
    let long_name = main
      .and_then(|(sub, _)| variants.get(&(line.to_string(), sub.to_string())))
      .and_then(|v| Some(format!("{} - {}", v.stops.first()?.name, v.stops.last()?.name)))
      .unwrap_or_default();
    let _ = writeln!(txt, "{line},codesa,{line},{},3", quote(&long_name));
  }
  files.push(("routes.txt", txt));

  let mut txt =
    String::from("service_id,monday,tuesday,wednesday,thursday,friday,saturday,sunday,start_date,end_date\n");
  for days in &services {
    let week = days.week().map(|on| u8::from(on).to_string()).join(",");
    let _ = writeln!(txt, "{},{week},{},{}", days.service_id(), VALIDITY.0, VALIDITY.1);
  }
  files.push(("calendar.txt", txt));

  let mut txt = String::from("shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\n");
  for (line, sub) in &shapes {
    let Some(coords) = variants.get(&(line.to_string(), sub.to_string())).and_then(|v| v.shape.as_ref()) else {
      continue;
    };
    for (seq, (lng, lat)) in coords.iter().enumerate() {
      let _ = writeln!(txt, "{line}_{sub},{lat},{lng},{seq}");
    }
  }
  files.push(("shapes.txt", txt));

  files.push(("trips.txt", format!("route_id,service_id,trip_id,direction_id,shape_id,trip_headsign\n{trip_lines}")));
  files.push(("stop_times.txt", format!("trip_id,arrival_time,departure_time,stop_id,stop_sequence\n{stop_times}")));

  let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
  let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
  for (name, body) in files {
    out.start_file(name, options).map_err(|e| Error::External(format!("{name}: {e}")))?;
    out.write_all(body.as_bytes()).map_err(|e| Error::External(format!("{name}: {e}")))?;
  }
  let cursor = out.finish().map_err(|e| Error::External(format!("couldn't close zip: {e}")))?;
  Ok(cursor.into_inner())
}

pub fn anchor(trip: &Trip, stops: &[Stop], table: &HashMap<String, Vec<&str>>) -> Vec<(usize, u32)> {
  let mut out: Vec<(usize, u32)> = Vec::new();
  let mut day_offset = 0;

  for passing in &trip.passings {
    let Some(codes) = table.get(&match_key(&passing.stop)) else {
      continue;
    };
    let from = out.last().map_or(0, |&(i, _)| i + 1);
    let Some(i) = stops.iter().enumerate().skip(from).find(|(_, p)| codes.contains(&p.code.as_str())).map(|(i, _)| i)
    else {
      continue;
    };

    let mut minute = passing.minute + day_offset;
    if let Some(&(_, previous)) = out.last() {
      if minute + 720 < previous {
        day_offset += 24 * 60;
        minute += 24 * 60;
      } else if minute < previous {
        continue;
      }
    }
    out.push((i, minute));
  }
  out
}

fn times(stops: &[Stop], anchors: &[(usize, u32)]) -> Vec<(usize, u32)> {
  let mut distance = vec![0.0; stops.len()];
  for i in 1..stops.len() {
    distance[i] = distance[i - 1] + stops[i - 1].location.distance_to(&stops[i].location);
  }

  let mut out = Vec::new();
  for pair in anchors.windows(2) {
    let [(a, ta), (b, tb)] = [pair[0], pair[1]];
    let (ta, tb) = (f64::from(ta * 60), f64::from(tb * 60));
    let span = distance[b] - distance[a];
    for i in a..b {
      let f = if span > 0.0 { (distance[i] - distance[a]) / span } else { (i - a) as f64 / (b - a) as f64 };
      out.push((i, (ta + f * (tb - ta)).round() as u32));
    }
  }
  if let Some(&(b, tb)) = anchors.last() {
    out.push((b, tb * 60));
  }
  out
}

fn clock(seconds: u32) -> String {
  format!("{:02}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

fn quote(s: &str) -> String {
  format!("\"{}\"", s.replace('"', "\"\""))
}
