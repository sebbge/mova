use mova_sdk as sdk;
use sdk::prelude::*;

mod api;
mod names;
mod provider;
use std::sync::Arc;

pub struct BusmatickConfig {
  pub base: String,
}

// maldonado, uruguay
pub fn codesa(config: BusmatickConfig) -> Result<Provider> {
  let feed = Arc::new(provider::Codesa::new(&config)?);
  let avl = Arc::new(provider::Busmatick::new("codesa", "CODESA", &config)?);
  let provider = Provider::new("codesa", Region::UyMa, &[Region::UyMa]).feed(feed).fleet(avl);
  Ok(provider)
}

pub fn mdoturismo(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("mdoturismo", "MALDONADO TURISMO", &config)?);
  let provider = Provider::new("mdoturismo", Region::UyMa, &[Region::UyMa]).fleet(avl);
  Ok(provider)
}

pub fn micro(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("micro", "MICRO", &config)?);
  let provider = Provider::new("micro", Region::UyMa, &[Region::UyMa]).fleet(avl);
  Ok(provider)
}

// paysandu, uruguay
pub fn copay(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::legacy_tls("copay", "COPAY", &config)?);
  let provider = Provider::new("copay", Region::UyPa, &[Region::UyPa]).fleet(avl);
  Ok(provider)
}

// rivera, uruguay
pub fn microrivera(config: BusmatickConfig) -> Result<Provider> {
  let avl = Arc::new(provider::Busmatick::new("microrivera", "MICRO", &config)?);
  let provider = Provider::new("microrivera", Region::UyRv, &[Region::UyRv]).fleet(avl);
  Ok(provider)
}

// colonia, uruguay
pub fn solantigua(config: BusmatickConfig) -> Result<Provider> {
  let sol = Arc::new(provider::Solantigua::new(&config)?);
  let provider = Provider::new("solantigua", Region::UyCo, &[Region::UyCo]).feed(sol.clone()).fleet(sol);
  Ok(provider)
}

pub mod test_support {
  pub use crate::api::codesa::{
    Stop, Trip, Variant, Variants, anchor, kml, parse_sheet, shape, stops, synthesize, table,
  };
  pub use crate::api::colonia::Shapes;
  pub use crate::api::parse_avl;
  pub use crate::names::match_key;
}
