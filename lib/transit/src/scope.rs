use crate::sdk::region::Region;
use crate::sdk::{arrivals::Arrivals, provider::Provider, vehicles::Vehicles};

use std::sync::Arc;

/// `region` issues the id, `provider` disambiguates
#[derive(Clone)]
pub struct Scope {
  pub region: Region,
  pub provider: &'static str,
  handle: Arc<Provider>,
}

impl Scope {
  pub fn new(handle: Arc<Provider>) -> Self {
    Self { region: handle.authority, provider: handle.id, handle }
  }

  pub fn vehicles(&self) -> Option<&Vehicles> {
    self.handle.vehicles.as_ref()
  }

  pub fn arrivals(&self) -> Option<&dyn Arrivals> {
    self.handle.arrivals.as_deref()
  }
}

pub fn qualify(provider: &str, id: &str) -> String {
  format!("{provider}-{id}")
}
