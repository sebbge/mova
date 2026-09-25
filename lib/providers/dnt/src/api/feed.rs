use super::kml::Shape;
use crate::core::{Error, Result};

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

pub fn rebuild(zip_bytes: Vec<u8>, shapes: Vec<Shape>) -> Result<Vec<u8>> {
  let by_variant: HashMap<String, Shape> = shapes.into_iter().map(|r| (r.variant.clone(), r)).collect();

  let mut archive =
    zip::ZipArchive::new(Cursor::new(zip_bytes)).map_err(|e| Error::External(format!("unreadable zip: {e}")))?;

  let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
  let options = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

  let mut trips = String::new();
  for i in 0..archive.len() {
    let mut entry = archive.by_index(i).map_err(|e| Error::External(format!("unreadable zip entry: {e}")))?;
    let name = entry.name().to_string();

    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).map_err(|e| Error::External(format!("zip read: {e}")))?;

    if name == "trips.txt" {
      trips = String::from_utf8_lossy(&buf).into_owned();
      continue;
    }

    write_entry(&mut out, &name, &buf, options)?;
  }

  let (trips_txt, used) = augment_trips(&trips, &by_variant);
  write_entry(&mut out, "trips.txt", trips_txt.as_bytes(), options)?;
  write_entry(&mut out, "shapes.txt", shapes_txt(&by_variant, &used).as_bytes(), options)?;

  let cursor = out.finish().map_err(|e| Error::External(format!("couldn't close zip: {e}")))?;
  Ok(cursor.into_inner())
}

fn write_entry(
  out: &mut zip::ZipWriter<Cursor<Vec<u8>>>,
  name: &str,
  bytes: &[u8],
  options: zip::write::SimpleFileOptions,
) -> Result<()> {
  out.start_file(name, options).map_err(|e| Error::External(format!("{name}: {e}")))?;
  out.write_all(bytes).map_err(|e| Error::External(format!("{name}: {e}")))?;
  Ok(())
}

fn augment_trips(trips: &str, by_variant: &HashMap<String, Shape>) -> (String, Vec<String>) {
  let mut out = String::from("route_id,service_id,trip_id,shape_id,direction_id,trip_headsign\n");
  let mut used = Vec::new();

  for line in trips.lines().skip(1).filter(|l| !l.trim().is_empty()) {
    let cols: Vec<String> = line.split(',').map(|c| c.trim().trim_matches('"').to_string()).collect();
    let [route_id, service_id, trip_id] = &cols[..] else {
      continue;
    };

    let (shape_id, direction, headsign) = match by_variant.get(route_id) {
      Some(r) => {
        used.push(route_id.clone());
        let direction = if r.direction == "B" { 1 } else { 0 };
        (route_id.as_str(), direction, r.destination.as_str())
      }
      None => ("", 0, ""),
    };

    out.push_str(&format!("{route_id},{service_id},{trip_id},{shape_id},{direction},\"{headsign}\"\n"));
  }

  used.sort_unstable();
  used.dedup();
  (out, used)
}

fn shapes_txt(by_variant: &HashMap<String, Shape>, used: &[String]) -> String {
  let mut out = String::from("shape_id,shape_pt_lat,shape_pt_lon,shape_pt_sequence\n");

  for variant in used {
    let Some(shape) = by_variant.get(variant) else {
      continue;
    };
    for (seq, (lng, lat)) in shape.coords.iter().enumerate() {
      out.push_str(&format!("{variant},{lat},{lng},{seq}\n"));
    }
  }

  out
}
