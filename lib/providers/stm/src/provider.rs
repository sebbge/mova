use crate::StmConfig;
use crate::api::StmApi;
use crate::sdk::prelude::*;

use std::sync::Arc;

#[derive(Clone)]
pub struct Stm {
  api: Arc<StmApi>,
}
impl Stm {
  pub fn new(config: StmConfig) -> Result<Self> {
    Ok(Self { api: Arc::new(StmApi::new(config)?) })
  }
}

#[async_trait::async_trait]
impl Feed for Stm {
  async fn version(&self) -> Result<String> {
    self.api.gtfs_version().await
  }

  async fn download(&self) -> Result<Static> {
    let (gtfs, headsigns) = tokio::join!(self.api.gtfs_download(), self.api.headsigns());
    let headsigns =
      headsigns.inspect_err(|e| tracing::warn!(error = ?e, "[stm/linevariants] GTFS headsigns kept")).ok();
    Ok(Static { gtfs: gtfs?, lines_by_stop: None, headsigns })
  }

  fn route(&self, raw: &str) -> String {
    crate::names::route_name(raw)
  }

  fn stop(&self, raw: &str) -> StopName {
    crate::names::stop_name(raw)
  }
}

#[async_trait::async_trait]
impl Fleet for Stm {
  async fn fleet(&self) -> Result<Vec<BusEntry>> {
    let (fleet, online) = tokio::join!(self.api.fleet(), self.api.online());
    match fleet {
      Ok(fleet) => {
        let mut buses: Vec<BusEntry> = fleet.into_iter().map(BusEntry::from).collect();
        let known: std::collections::HashSet<String> = buses.iter().map(|b| b.id.clone()).collect();
        let now = chrono::Utc::now();
        let missing = online.unwrap_or_default().into_iter().map(BusEntry::from).filter(|b| !known.contains(&b.id));
        buses.extend(missing.map(|b| BusEntry { timestamp: Some(now), ..b }));
        Ok(buses)
      }
      Err(e) => {
        tracing::warn!(error = ?e, "[stm/online] fleet fallback");
        Ok(online?.into_iter().map(BusEntry::from).collect())
      }
    }
  }
}

#[async_trait::async_trait]
impl Arrivals for Stm {
  async fn upcoming(&self, stop_id: &str, lines: &[String]) -> Result<UpcomingByLine> {
    let by_line = self.api.upcoming_lines(stop_id, lines).await?;
    Ok(
      by_line.into_iter().map(|(line, buses)| (line, buses.into_iter().map(UpcomingArrival::from).collect())).collect(),
    )
  }
}
