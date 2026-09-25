use crate::core::provider::{Arrivals, Provider, Vehicles};

use std::sync::Arc;

/// `region` issues the id, `provider` disambiguates
#[derive(Clone)]
pub struct Scope {
  pub region: String,
  pub provider: String,
  handle: Arc<Provider>,
}

impl Scope {
  pub fn new(handle: Arc<Provider>) -> Self {
    let region = handle.authority.id().to_string();
    let provider = handle.id.to_string();
    Self { region, provider, handle }
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
