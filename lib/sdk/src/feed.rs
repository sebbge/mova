use crate::{Result, format};

use std::collections::HashMap;

pub type LinesByStop = HashMap<String, Vec<String>>;
pub type HeadsignsByShape = HashMap<String, String>;

pub struct Static {
  pub gtfs: Vec<u8>,
  pub lines_by_stop: Option<LinesByStop>,
  /// Destination per shape, overrides `trip_headsign`
  pub headsigns: Option<HeadsignsByShape>,
}

pub struct StopName {
  pub name: String,
  pub platform: Option<Platform>,
}

pub struct Platform {
  pub station: String,
  pub code: String,
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

  fn stop(&self, raw: &str) -> StopName {
    StopName { name: self.place(raw), platform: None }
  }
}
