use crate::api::StmApi;
use crate::api::models::{StmBus, StmLineBus, StmUpcomingBus};
use citymob_core as core;
use core::Result;
use core::bus::{BusEntry, UpcomingArrival};
use core::provider::{BusesByLine, Feed, Names, Provider, Realtime, UpcomingByLine};

pub struct Stm {
  api: StmApi,
}

impl Stm {
  pub fn new() -> Result<Self> {
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();

    let api = StmApi::new(var("IM_API_URL")?, var("IM_AUTH_URL")?, var("IM_CLIENT_ID")?, var("IM_CLIENT_SECRET")?);
    Ok(Self { api })
  }
}

fn var(key: &str) -> Result<String> {
  std::env::var(key).map_err(|_| core::Error::Config(format!("uy-mo: falta {key}")))
}

impl Provider for Stm {
  fn regions(&self) -> &[&str] {
    &["uy-mo"]
  }
}

#[async_trait::async_trait]
impl Realtime for Stm {
  async fn buses(&self, lines: &[String]) -> Result<BusesByLine> {
    let by_line = self.api.lines(lines).await?;
    Ok(by_line.into_iter().map(|(line, buses)| (line, buses.into_iter().map(observe_line_bus).collect())).collect())
  }

  async fn upcoming(&self, stop_id: &str, lines: &[String]) -> Result<UpcomingByLine> {
    let by_line = self.api.upcoming_lines(stop_id, lines).await?;
    Ok(by_line.into_iter().map(|(line, buses)| (line, buses.into_iter().map(observe_upcoming).collect())).collect())
  }

  async fn probe(&self) -> bool {
    self.api.probe().await
  }
}

#[async_trait::async_trait]
impl Feed for Stm {
  async fn version(&self) -> Result<String> {
    self.api.gtfs_version().await
  }

  async fn download(&self) -> Result<Vec<u8>> {
    self.api.gtfs_download().await
  }
}

impl Names for Stm {
  fn place(&self, raw: &str) -> String {
    core::text::place_name(raw)
  }

  fn route(&self, raw: &str) -> String {
    crate::names::route_name(raw)
  }

  fn line(&self, raw: &str) -> String {
    crate::names::line_name(raw)
  }
}

fn observe(bus: StmBus) -> BusEntry {
  BusEntry {
    id: bus.bus_id.to_string(),
    line: bus.line,
    company: bus.company,
    location: bus.location,
    speed: None,
    origin: Some(core::text::place_name(&bus.origin)),
    destination: Some(core::text::place_name(&bus.destination)),
    timestamp: None,
    access: bus.access.label(),
    emissions: bus.emissions.label(),
    thermal_comfort: bus.thermal_confort.label(),
  }
}

fn observe_line_bus(im: StmLineBus) -> BusEntry {
  BusEntry { speed: im.speed, timestamp: im.timestamp, ..observe(im.bus) }
}

fn observe_upcoming(im: StmUpcomingBus) -> UpcomingArrival {
  UpcomingArrival { eta_s: im.eta, distance_m: im.distance, position: im.position, bus: observe(im.bus) }
}
