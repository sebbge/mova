// data/lugares.csv, curated with .project/sources/inter/scripts/lugares.py
use crate::core::provider::prelude::*;

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

const CSV: &str = include_str!("../../data/lugares.csv");

pub struct Place {
  pub name: &'static str,
  pub location: Point,
}

pub fn all() -> HashMap<&'static str, Place> {
  CSV
    .lines()
    .skip(1)
    .filter_map(|line| {
      let cols: Vec<&str> = line.split(';').collect();
      let [code, locality, stop, _, _, lat, lon, ..] = cols[..] else {
        return None;
      };
      let location = Point { lat: lat.parse().ok()?, lng: lon.parse().ok()? };
      let name = if stop.is_empty() { locality } else { stop };
      Some((code, Place { name, location }))
    })
    .collect()
}

pub fn fingerprint() -> u64 {
  let mut hasher = DefaultHasher::new();
  CSV.hash(&mut hasher);
  hasher.finish()
}
