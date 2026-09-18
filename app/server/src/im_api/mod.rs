// https://api.montevideo.gub.uy/apidocs/publictransport
mod auth;
use auth::ImAuthClient;

pub mod models;
use models::{ImLineBus, ImUpcomingBus};

use std::collections::HashMap;

pub(super) struct ImApi {
  http: reqwest::Client,
  url: String,
  auth: ImAuthClient,
}
impl ImApi {
  pub(super) fn new(url: String, auth_url: String, client_id: String, client_secret: String) -> Self {
    let http = reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(12))
      .pool_max_idle_per_host(0)
      .build()
      .expect("failed to build HTTP client");
    let auth = ImAuthClient::new(auth_url, client_id, client_secret);
    Self { http, url, auth }
  }
}

//# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses?lines={lines}&amountperline=3
// - upcoming buses
pub type ImUpcomingLines = HashMap<String, Vec<ImUpcomingBus>>;
pub trait UpcomingLinesData {
  async fn upcoming_lines(&self, stop_id: &str, lines: &[String]) -> crate::Result<ImUpcomingLines>;
}
impl UpcomingLinesData for ImApi {
  async fn upcoming_lines(&self, stop_id: &str, lines: &[String]) -> crate::Result<ImUpcomingLines> {
    let lines = lines.join(",");
    let url = format!("{}/buses/busstops/{}/upcomingbuses?lines={}&amountperline=3", self.url, stop_id, lines);

    let token = self.auth.token(&self.http).await?;
    match self.http.get(&url).bearer_auth(&token).send().await {
      Ok(res) => {
        let status = res.status();
        if status.is_success() {
          let upcomings: Vec<ImUpcomingBus> = res.json().await.unwrap_or_default();

          let total = if lines.is_empty() { 0 } else { lines.split(',').count() };
          tracing::info!(stop = stop_id, lines = total, buses = upcomings.len(), "[MVD/upcomingbuses] OK");

          let mut by_line = ImUpcomingLines::new();
          for u in upcomings {
            by_line.entry(u.bus.line.clone()).or_default().push(u);
          }
          Ok(by_line)
        } else {
          tracing::warn!(status = status.as_u16(), url = %url, "[MVD/upcomingbuses] non-2xx — using GTFS only");
          Ok(ImUpcomingLines::new())
        }
      }
      Err(e) => {
        tracing::warn!(error = %e, url = %url, "[MVD/upcomingbuses] request failed — using GTFS only");
        Err(e.into())
      }
    }
  }
}

//# GET <MVD>/buses?lines={lines}
// - realtime location & ETA
pub type ImLines = HashMap<String, Vec<ImLineBus>>;
pub trait LinesData {
  async fn lines(&self, lines: &[String]) -> crate::Result<ImLines>;
}
impl LinesData for ImApi {
  async fn lines(&self, lines: &[String]) -> crate::Result<ImLines> {
    let lines = lines.join(",");
    let url = format!("{}/buses?lines={lines}", self.url);
    let token = self.auth.token(&self.http).await?;

    let res = self.http.get(&url).bearer_auth(&token).send().await?;
    let status = res.status();

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
      tracing::warn!(url = %url, "[MVD/buses] rate limited (429)");
      return Err(crate::Error::RateLimited);
    }

    if !status.is_success() {
      let body = res.text().await.unwrap_or_default();
      tracing::warn!(status = status.as_u16(), url = %url, body = %body, "[MVD/buses] non-2xx");
      return Ok(ImLines::new());
    }

    let bytes = res.bytes().await.map_err(|e| {
      tracing::error!(url = %url, error = %e, "[MVD/buses] failed to read response body");
      crate::Error::ExternalApi(format!("body read error at {url}: {e}"))
    })?;

    let flat: Vec<ImLineBus> = serde_json::from_slice(&bytes).map_err(|e| {
      let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
      tracing::error!(url = %url, error = %e, body = %snippet, "[MVD/buses] JSON decode failed");
      crate::Error::ExternalApi(format!("JSON decode at {url}: {e}"))
    })?;

    let total = if lines.is_empty() { 0 } else { lines.split(',').count() };
    tracing::info!(lines = total, buses = flat.len(), "[MVD/buses] OK");

    let mut by_line = ImLines::new();
    for bus in flat {
      by_line.entry(bus.bus.line.clone()).or_default().push(bus);
    }
    Ok(by_line)
  }
}

//# GET <MVD>/buses/gtfs/static/latest/...
// - GTFS static data
//- https://gtfs.org/
pub trait GtfsData {
  const URL: &str = "/buses/gtfs/static/latest";
  async fn gtfs_version(&self) -> crate::Result<String>;
  async fn gtfs_download(&self) -> crate::Result<Vec<u8>>;
}
impl GtfsData for ImApi {
  async fn gtfs_version(&self) -> crate::Result<String> {
    let token = self.auth.token(&self.http).await?;
    let url = format!("{}{}/version.txt", self.url, Self::URL);

    let res = self
      .http
      .get(&url)
      .bearer_auth(&token)
      .send()
      .await
      .map_err(|e| crate::Error::ExternalApi(format!("GTFS version request failed: {e}")))?;

    if !res.status().is_success() {
      return Err(crate::Error::ExternalApi(format!("GTFS version returned HTTP {}", res.status())));
    }

    let version = res.text().await.map_err(|e| crate::Error::ExternalApi(format!("GTFS version read failed: {e}")))?;
    Ok(version.trim().to_string())
  }

  async fn gtfs_download(&self) -> crate::Result<Vec<u8>> {
    let token = self.auth.token(&self.http).await?;
    let url = format!("{}{}/google_transit.zip", self.url, Self::URL);

    let res = self
      .http
      .get(&url)
      .bearer_auth(&token)
      .send()
      .await
      .map_err(|e| crate::Error::ExternalApi(format!("GTFS ZIP request failed: {e}")))?;

    if !res.status().is_success() {
      return Err(crate::Error::ExternalApi(format!("GTFS ZIP download returned HTTP {}", res.status())));
    }

    res.bytes().await.map(|b| b.to_vec()).map_err(|e| crate::Error::ExternalApi(format!("ZIP read failed: {e}")))
  }
}

//# GET <MVD>/buses?lines={test_line}
// - realtime API health
pub trait HealthProbe {
  async fn probe(&self) -> bool;
}
impl HealthProbe for ImApi {
  async fn probe(&self) -> bool {
    let Ok(token) = self.auth.token(&self.http).await else { return false };
    let url = format!("{}/buses?lines=151", self.url);
    match self.http.get(&url).bearer_auth(token).send().await {
      Ok(resp) => resp.status().is_success() || resp.status().as_u16() < 500,
      _ => false,
    }
  }
}
