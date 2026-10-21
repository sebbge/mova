mod fare;
mod query;
mod raptor;
mod timetable;

#[cfg(test)]
mod tests;

pub use fare::{Fare, Rider};
pub use query::{Itinerary, Leg, Mode, Place, Query};
use raptor::Raptor;

use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
  NotReady,
  Transit(mova_transit::Error),
  Internal(String),
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<mova_transit::Error> for Error {
  fn from(e: mova_transit::Error) -> Self {
    Self::Transit(e)
  }
}

pub struct Planner {
  raptor: Arc<Raptor>,
}

impl Planner {
  pub fn start(transit: mova_transit::Transit) -> Self {
    let raptor = Arc::new(Raptor::new(transit));
    raptor.clone().spawn();
    Self { raptor }
  }

  pub async fn plan(&self, query: Query) -> Result<Vec<Itinerary>> {
    self.raptor.plan(query).await.inspect_err(|e| tracing::warn!(error = ?e, "[planner/plan] raptor failed"))
  }
}
