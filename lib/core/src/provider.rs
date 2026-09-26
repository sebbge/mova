use crate::bus::{BusEntry, UpcomingArrival};
use crate::region::Region;
use crate::{Result, format};

use std::collections::HashMap;
use std::sync::Arc;

pub type BusesByLine = HashMap<String, Vec<BusEntry>>;
pub type UpcomingByLine = HashMap<String, Vec<UpcomingArrival>>;
pub type LinesByStop = HashMap<String, Vec<String>>;

pub struct Provider {
  pub id: &'static str,
  pub authority: Region,
  pub regions: &'static [Region],
  pub feed: Option<Arc<dyn Feed>>,
  pub vehicles: Option<Vehicles>,
  pub arrivals: Option<Arc<dyn Arrivals>>,
}

impl Provider {
  pub fn new(id: &'static str, authority: Region, regions: &'static [Region]) -> Self {
    Self { id, authority, regions, feed: None, vehicles: None, arrivals: None }
  }

  pub fn feed(self, feed: Arc<dyn Feed>) -> Self {
    Self { feed: Some(feed), ..self }
  }

  pub fn fleet(self, fleet: Arc<dyn Fleet>) -> Self {
    Self { vehicles: Some(Vehicles::Fleet(fleet)), ..self }
  }

  pub fn lines(self, lines: Arc<dyn LineBuses>) -> Self {
    Self { vehicles: Some(Vehicles::Lines(lines)), ..self }
  }

  pub fn arrivals(self, arrivals: Arc<dyn Arrivals>) -> Self {
    Self { arrivals: Some(arrivals), ..self }
  }
}

pub struct Static {
  pub gtfs: Vec<u8>,
  pub lines_by_stop: Option<LinesByStop>,
}

#[async_trait::async_trait]
pub trait Feed: Send + Sync {
  async fn version(&self) -> Result<String>;
  async fn download(&self) -> Result<Static>;

  fn place(&self, raw: &str) -> String {
    format::place_name(raw)
  }
  fn route(&self, raw: &str) -> String {
    format::place_name(raw)
  }
  fn line(&self, raw: &str) -> String {
    format::line_name(raw)
  }
}

#[derive(Clone)]
pub enum Vehicles {
  Fleet(Arc<dyn Fleet>),
  Lines(Arc<dyn LineBuses>),
}

#[async_trait::async_trait]
pub trait Fleet: Send + Sync {
  async fn fleet(&self) -> Result<Vec<BusEntry>>;
}

#[async_trait::async_trait]
pub trait LineBuses: Send + Sync {
  async fn buses(&self, lines: &[String]) -> Result<BusesByLine>;
  async fn probe(&self) -> bool;
}

#[async_trait::async_trait]
pub trait Arrivals: Send + Sync {
  async fn upcoming(&self, stop_id: &str, lines: &[String]) -> Result<UpcomingByLine>;
}

pub mod prelude {
  pub use super::{
    Arrivals, BusesByLine, Feed, Fleet, LineBuses, LinesByStop, Provider, Static, UpcomingByLine, Vehicles,
  };
  pub use crate::bus::{BusEntry, StopRef, UpcomingArrival};
  pub use crate::geo::Point;
  pub use crate::region::Region;
  pub use crate::{Error, Result, format, http};
}
