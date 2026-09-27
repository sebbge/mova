use citymob_nexus_core as core;
use core::provider::prelude::*;

mod api;
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
      .arrivals(stm),
  )
}
