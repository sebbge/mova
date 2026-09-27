use crate::names::match_key;

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

const CSV: &str = include_str!("../../../data/paradas.csv");

pub fn table() -> HashMap<String, Vec<&'static str>> {
  CSV
    .lines()
    .skip(1)
    .filter_map(|line| {
      let (name, codes) = line.split_once(';')?;
      Some((match_key(name), codes.split_whitespace().collect()))
    })
    .collect()
}

pub fn fingerprint() -> u64 {
  let mut hasher = DefaultHasher::new();
  CSV.hash(&mut hasher);
  hasher.finish()
}
