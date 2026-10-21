use mova_sdk as sdk;
use sdk::prelude::*;

mod api;
mod fares;
mod names;
mod provider;
use std::sync::Arc;

pub struct StmConfig {
  pub api_url: String,
  pub auth_url: String,
  pub client_id: String,
  pub client_secret: String,
}

pub fn provider(config: StmConfig) -> Result<Provider> {
  let stm = Arc::new(provider::Stm::new(config)?);
  Ok(
    Provider::new("stm", Region::UyMo, &[Region::UyMo, Region::UyCa, Region::UySj])
      .feed(stm.clone())
      .fleet(stm.clone())
      .arrivals(stm)
      .fares(Arc::new(fares::StmFares::load()?)),
  )
}
