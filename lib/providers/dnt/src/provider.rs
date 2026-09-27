use super::api::{self, DntApi, Index};
use crate::sdk::prelude::*;

use std::sync::Arc;
use tokio::sync::RwLock;

pub struct Dnt {
  api: DntApi,
}
impl Dnt {
  pub fn new() -> Result<Self> {
    Ok(Self { api: DntApi::new()? })
  }
}

#[async_trait::async_trait]
impl Feed for Dnt {
  async fn version(&self) -> Result<String> {
    self.api.gtfs_version().await
  }

  async fn download(&self) -> Result<Static> {
    let (gtfs, roster) = tokio::join!(self.api.gtfs_download(), self.api.lines_by_stop());
    let lines_by_stop =
      roster.inspect_err(|e| tracing::warn!(error = ?e, "[dnt/roster] unavailable, continuing without it")).ok();

    Ok(Static { gtfs: gtfs?, lines_by_stop })
  }

  fn route(&self, raw: &str) -> String {
    crate::names::route_name(raw)
  }
}

#[async_trait::async_trait]
impl Fleet for Dnt {
  async fn fleet(&self) -> Result<Vec<BusEntry>> {
    Ok(self.api.buses().await?.into_iter().map(BusEntry::from).collect())
  }
}

pub struct Inter {
  api: DntApi,
  index: RwLock<Option<Arc<Index>>>,
}
impl Inter {
  pub fn new() -> Result<Self> {
    Ok(Self { api: DntApi::new()?, index: RwLock::new(None) })
  }

  async fn index(&self) -> Result<Arc<Index>> {
    if let Some(index) = self.index.read().await.as_ref() {
      return Ok(index.clone());
    }
    let trips = self.api.long_distance().await?;
    let index = Arc::new(Index::new(&trips, &api::corridors(&trips)));
    *self.index.write().await = Some(index.clone());
    Ok(index)
  }
}

#[async_trait::async_trait]
impl Feed for Inter {
  async fn version(&self) -> Result<String> {
    self.api.long_distance_version().await
  }

  async fn download(&self) -> Result<Static> {
    let trips = self.api.long_distance().await?;
    let (gtfs, index) = tokio::task::spawn_blocking(move || {
      let corridors = api::corridors(&trips);
      let gtfs = api::synthesize(&trips, &corridors, &api::places())?;
      Ok::<_, Error>((gtfs, Index::new(&trips, &corridors)))
    })
    .await
    .map_err(|e| Error::External(format!("feed synthesis panicked: {e}")))??;

    *self.index.write().await = Some(Arc::new(index));
    Ok(Static { gtfs, lines_by_stop: None })
  }
}

#[async_trait::async_trait]
impl Fleet for Inter {
  async fn fleet(&self) -> Result<Vec<BusEntry>> {
    let (services, index) = tokio::join!(self.api.services(), self.index());
    let (services, index) = (services?, index?);

    let total = services.len();
    let buses: Vec<BusEntry> = services.into_iter().filter_map(|s| s.into_bus(&index)).collect();
    tracing::debug!(matched = buses.len(), unmatched = total - buses.len(), "[inter/match] services reconciled");
    Ok(buses)
  }
}
