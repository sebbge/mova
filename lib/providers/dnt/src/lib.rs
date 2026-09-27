use citymob_nexus_core as core;
use core::provider::prelude::*;

mod api;
mod names;
mod provider;
use std::sync::Arc;

const URUGUAY: &[Region] = &[
  Region::UyAr,
  Region::UyCa,
  Region::UyCl,
  Region::UyCo,
  Region::UyDu,
  Region::UyFd,
  Region::UyFs,
  Region::UyLa,
  Region::UyMa,
  Region::UyMo,
  Region::UyPa,
  Region::UyRn,
  Region::UyRo,
  Region::UyRv,
  Region::UySa,
  Region::UySj,
  Region::UySo,
  Region::UyTa,
  Region::UyTt,
];

pub fn metropolitan() -> Result<Provider> {
  let dnt = Arc::new(provider::Dnt::new()?);
  Ok(
    Provider::new("dnt", Region::UyMo, &[Region::UyMo, Region::UyCa, Region::UySj, Region::UyFd])
      .feed(dnt.clone())
      .fleet(dnt),
  )
}

pub fn interdepartmental() -> Result<Provider> {
  let inter = Arc::new(provider::Inter::new()?);
  Ok(Provider::new("inter", Region::Uy, URUGUAY).feed(inter.clone()).fleet(inter))
}

pub mod test_support {
  pub use crate::api::{
    Index, Service, Shape, corridors, fetch_roster, parse_kml, parse_long_distance, parse_roster, places, rebuild_feed,
    synthesize,
  };
}
