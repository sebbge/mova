use super::{StmConfig, api::StmApi};
use crate::sdk::prelude::*;

pub struct Stm {
  api: StmApi,
}
impl Stm {
  pub fn new(config: StmConfig) -> Result<Self> {
    let StmConfig { api_url, auth_url, client_id, client_secret } = config;

    for (key, value) in [
      ("IM_API_URL", &api_url),
      ("IM_AUTH_URL", &auth_url),
      ("IM_CLIENT_ID", &client_id),
      ("IM_CLIENT_SECRET", &client_secret),
    ] {
      if value.trim().is_empty() {
        return Err(Error::Config(format!("stm: missing {key}")));
      }
    }

    Ok(Self { api: StmApi::new(api_url, auth_url, client_id, client_secret)? })
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
    match self.api.fleet().await {
      Ok(fleet) => Ok(fleet.into_iter().map(BusEntry::from).collect()),
      Err(e) => {
        tracing::warn!(error = ?e, "[stm/online] fleet fallback");
        Ok(self.api.online().await?.into_iter().map(BusEntry::from).collect())
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
