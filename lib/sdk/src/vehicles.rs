use crate::Result;
use crate::geo::Point;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::collections::HashMap;
use std::sync::Arc;

pub type BusesByLine = HashMap<String, Vec<BusEntry>>;

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

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BusEntry {
  pub timestamp: Option<DateTime<Utc>>,

  pub id: String,
  pub unit: String,
  pub line: String,
  pub company: String,
  pub location: Point,
  pub speed: Option<f64>,

  pub shape_id: Option<String>,
  pub origin: Option<String>,
  pub destination: Option<String>,
  pub delay_min: Option<i64>,
  pub stop: Option<StopRef>,
  pub next_stop: Option<StopRef>,

  pub access: Option<String>,
  pub emissions: Option<String>,
  pub thermal_comfort: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopRef {
  pub id: String,
  pub name: String,
}

impl BusEntry {
  pub fn identity(company: &str, unit: &str) -> String {
    format!("{company}:{unit}")
  }
}
