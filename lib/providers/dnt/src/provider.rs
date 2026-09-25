use super::api::DntApi;
use crate::core::provider::prelude::*;

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
