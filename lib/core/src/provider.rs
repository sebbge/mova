use crate::Result;
use crate::bus::{BusEntry, UpcomingArrival};

use std::collections::HashMap;

pub type BusesByLine = HashMap<String, Vec<BusEntry>>;
pub type UpcomingByLine = HashMap<String, Vec<UpcomingArrival>>;

#[async_trait::async_trait]
pub trait Realtime {
  async fn buses(&self, lines: &[String]) -> Result<BusesByLine>;
  async fn upcoming(&self, stop_id: &str, lines: &[String]) -> Result<UpcomingByLine>;
  async fn probe(&self) -> bool;
}

#[async_trait::async_trait]
pub trait Feed {
  async fn version(&self) -> Result<String>;
  async fn download(&self) -> Result<Vec<u8>>;
}

pub trait Names: Send + Sync {
  fn place(&self, raw: &str) -> String;
  fn route(&self, raw: &str) -> String;
  fn line(&self, raw: &str) -> String;
}

pub trait Provider: Realtime + Feed + Names + Send + Sync {
  /// ISO 3166-2, several per provider
  fn regions(&self) -> &[&str];
}
