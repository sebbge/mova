// <base>/Paradas/{line}_{subline}.kmz
// <base>/Recorridos/{line}_{subline}.kmz
use crate::sdk::prelude::*;

use std::io::{Cursor, Read};

pub struct Stop {
  pub code: String,
  pub name: String,
  pub location: Point,
}

pub fn kml(bytes: &[u8]) -> Result<String> {
  if !bytes.starts_with(b"PK") {
    return Ok(String::from_utf8_lossy(bytes).into_owned());
  }

  let mut archive =
    zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| Error::External(format!("unreadable kmz: {e}")))?;
  for i in 0..archive.len() {
    let mut entry = archive.by_index(i).map_err(|e| Error::External(format!("unreadable kmz entry: {e}")))?;
    if entry.name().ends_with(".kml") {
      let mut text = String::new();
      entry.read_to_string(&mut text).map_err(|e| Error::External(format!("kmz read: {e}")))?;
      return Ok(text);
    }
  }
  Err(Error::External("kmz without a kml inside".into()))
}

pub fn stops(kml: &str) -> Vec<Stop> {
  kml
    .split("<Placemark>")
    .skip(1)
    .filter_map(|block| {
      let label = unescape(between(block, "<name>", "</name>")?);
      let (code, name) = crate::names::split_stop(&label)?;
      let (lng, lat) = *coordinates(between(block, "<coordinates>", "</coordinates>")?).first()?;
      Some(Stop { code: code.to_string(), name: name.to_string(), location: Point { lat, lng } })
    })
    .collect()
}

pub fn shape(kml: &str) -> Option<Vec<(f64, f64)>> {
  let line = between(kml, "<LineString>", "</LineString>")?;
  let coords = coordinates(between(line, "<coordinates>", "</coordinates>")?);
  (coords.len() >= 2).then_some(coords)
}

fn between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
  let start = haystack.find(open)? + open.len();
  let rest = &haystack[start..];
  Some(&rest[..rest.find(close)?])
}

fn coordinates(raw: &str) -> Vec<(f64, f64)> {
  raw
    .split_whitespace()
    .filter_map(|point| {
      let mut parts = point.split(',');
      let lng: f64 = parts.next()?.parse().ok()?;
      let lat: f64 = parts.next()?.parse().ok()?;
      (lng != 0.0 && lat != 0.0).then_some((lng, lat))
    })
    .collect()
}

fn unescape(raw: &str) -> String {
  raw.replace("&apos;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}
