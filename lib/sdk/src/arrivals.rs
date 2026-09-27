use crate::Result;
use crate::vehicles::BusEntry;

use std::collections::HashMap;

pub type UpcomingByLine = HashMap<String, Vec<UpcomingArrival>>;

#[async_trait::async_trait]
pub trait Arrivals: Send + Sync {
  async fn upcoming(&self, stop_id: &str, lines: &[String]) -> Result<UpcomingByLine>;
}

#[derive(Clone, Debug)]
pub struct UpcomingArrival {
  pub bus: BusEntry,
  pub eta_s: Option<i64>,
  pub distance_m: Option<i64>,
  pub position: Option<i64>,
}
