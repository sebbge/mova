use citymob_core as core;
use core::provider::prelude::*;

mod api;
mod names;
mod provider;
use std::sync::Arc;

pub fn provider() -> Result<Provider> {
  let dnt = Arc::new(provider::Dnt::new()?);
  Ok(
    Provider::new("dnt", Region::UyMo, &[Region::UyMo, Region::UyCa, Region::UySj, Region::UyFd])
      .feed(dnt.clone())
      .fleet(dnt),
  )
}

pub mod test_support {
  pub use crate::api::{Shape, fetch_roster, parse_kml, parse_roster, rebuild_feed};
}
