use mova_sdk as sdk;
use sdk::prelude::*;

mod api;
mod names;
mod provider;

pub struct BusmatickConfig {
  pub base: String,
}

// maldonado, uruguay
pub fn codesa(config: BusmatickConfig) -> Result<Provider> {
  let feed = provider::Codesa::new(&config)?;
  let avl = provider::Busmatick::new("codesa", "CODESA", &config)?;
  Ok(Provider::new("codesa", Region::UyMa, &[Region::UyMa]).feed(feed).fleet(avl))
}

pub fn mdoturismo(config: BusmatickConfig) -> Result<Provider> {
  let avl = provider::Busmatick::new("mdoturismo", "MALDONADO TURISMO", &config)?;
  Ok(Provider::new("mdoturismo", Region::UyMa, &[Region::UyMa]).fleet(avl))
}

pub fn micro(config: BusmatickConfig) -> Result<Provider> {
  let avl = provider::Busmatick::new("micro", "MICRO", &config)?;
  Ok(Provider::new("micro", Region::UyMa, &[Region::UyMa]).fleet(avl))
}

// paysandu, uruguay
pub fn copay(config: BusmatickConfig) -> Result<Provider> {
  let avl = provider::Busmatick::legacy_tls("copay", "COPAY", &config)?;
  Ok(Provider::new("copay", Region::UyPa, &[Region::UyPa]).fleet(avl))
}

// rivera, uruguay
pub fn microrivera(config: BusmatickConfig) -> Result<Provider> {
  let avl = provider::Busmatick::new("microrivera", "MICRO", &config)?;
  Ok(Provider::new("microrivera", Region::UyRv, &[Region::UyRv]).fleet(avl))
}

// colonia, uruguay
pub fn solantigua(config: BusmatickConfig) -> Result<Provider> {
  let sol = provider::Solantigua::new(&config)?;
  Ok(Provider::new("solantigua", Region::UyCo, &[Region::UyCo]).feed(sol.clone()).fleet(sol))
}
