use citymob_nexus_core as core;
use core::provider::prelude::*;

mod api;
mod names;
mod provider;
use std::sync::Arc;

const MALDONADO: &[Region] = &[Region::UyMa];
const COLONIA: &[Region] = &[Region::UyCo];
const PAYSANDU: &[Region] = &[Region::UyPa];
const RIVERA: &[Region] = &[Region::UyRv];

pub struct BusmatickConfig {
  pub base: String,
}

pub fn codesa(config: BusmatickConfig) -> Result<Provider> {
  let feed = Arc::new(provider::Codesa::new(&config)?);
  let avl = Arc::new(provider::Busmatick::new("codesa", "CODESA", &config)?);
  Ok(Provider::new("codesa", Region::UyMa, MALDONADO).feed(feed).fleet(avl))
}

pub fn mdoturismo(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("mdoturismo", "MALDONADO TURISMO", &config)?);
  Ok(Provider::new("mdoturismo", Region::UyMa, MALDONADO).fleet(avl))
}

pub fn micro(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("micro", "MICRO", &config)?);
  Ok(Provider::new("micro", Region::UyMa, MALDONADO).fleet(avl))
}

pub fn copay(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::legacy_tls("copay", "COPAY", &config)?);
  Ok(Provider::new("copay", Region::UyPa, PAYSANDU).fleet(avl))
}

pub fn microrivera(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("microrivera", "MICRO", &config)?);
  Ok(Provider::new("microrivera", Region::UyRv, RIVERA).fleet(avl))
}

pub fn solantigua(config: BusmatickConfig) -> Result<Provider> {
  let sol = Arc::new(provider::Solantigua::new(&config)?);
  Ok(Provider::new("solantigua", Region::UyCo, COLONIA).feed(sol.clone()).fleet(sol))
}

pub mod test_support {
  pub use crate::api::codesa::{
    Stop, Trip, Variant, Variants, anchor, kml, parse_sheet, shape, stops, synthesize, table,
  };
  pub use crate::api::colonia::Shapes;
  pub use crate::api::parse_avl;
  pub use crate::names::match_key;
}
