// https://api.montevideo.gub.uy/apidocs/publictransport
mod auth;
pub mod models;
use self::auth::ImAuthClient;
use self::models::{StmLineBus, StmUpcomingBus};

use std::collections::{HashMap, HashSet};

pub struct StmApi {
  http: reqwest::Client,
  url: String,
  auth: ImAuthClient,
}
impl StmApi {
  #[allow(clippy::expect_used)]
  pub fn new(url: String, auth_url: String, client_id: String, client_secret: String) -> Self {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let http = reqwest::Client::builder()
      .timeout(std::time::Duration::from_secs(12))
      .build()
      .expect("failed to build HTTP client");
    let auth = ImAuthClient::new(auth_url, client_id, client_secret);
    Self { http, url, auth }
  }
}

//# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses?lines={lines}&amountperline=3
// - upcoming buses

pub type ImUpcomingLines = HashMap<String, Vec<StmUpcomingBus>>;
impl StmApi {
  async fn upcoming_chunk(&self, stop_id: &str, lines: &[String]) -> crate::core::Result<Vec<StmUpcomingBus>> {
    let url =
      format!("{}/buses/busstops/{}/upcomingbuses?lines={}&amountperline=3", self.url, stop_id, lines.join(","));

    let token = self.auth.token(&self.http).await?;
    let res = match self.http.get(&url).bearer_auth(&token).send().await {
      Ok(res) => res,
      Err(e) => {
        tracing::warn!(error = %e, url = %url, "[MVD/upcomingbuses] request failed");
        return Err(e.into());
      }
    };

    let status = res.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
      tracing::warn!(url = %url, "[MVD/upcomingbuses] rate limited (429)");
      return Err(crate::core::Error::RateLimited);
    }
    if !status.is_success() {
      let body = res.text().await.unwrap_or_default();
      tracing::warn!(status = status.as_u16(), url = %url, body = %body, "[MVD/upcomingbuses] non-2xx");
      return Err(crate::core::Error::External(format!("{status} at {url}")));
    }

    Ok(res.json().await.unwrap_or_default())
  }
}
impl StmApi {
  const UPCOMING_CHUNK: usize = 8;
  const UPCOMING_CONCURRENCY: usize = 3;

  pub async fn upcoming_lines(&self, stop_id: &str, lines: &[String]) -> crate::core::Result<ImUpcomingLines> {
    use futures_util::StreamExt;

    let pending: Vec<_> = lines.chunks(Self::UPCOMING_CHUNK).map(|chunk| self.upcoming_chunk(stop_id, chunk)).collect();
    let total = pending.len();

    let results: Vec<_> =
      futures_util::stream::iter(pending).buffer_unordered(Self::UPCOMING_CONCURRENCY).collect().await;

    let requested: HashSet<&str> = lines.iter().map(String::as_str).collect();

    let mut by_line = ImUpcomingLines::new();
    let mut failed = 0;
    let mut buses = 0;
    let mut ignored = 0;
    for result in results {
      match result {
        Ok(upcomings) => {
          buses += upcomings.len();
          for u in upcomings {
            if requested.contains(u.bus.line.as_str()) {
              by_line.entry(u.bus.line.clone()).or_default().push(u);
            } else {
              ignored += 1;
            }
          }
        }
        Err(_) => failed += 1,
      }
    }

    if ignored > 0 {
      tracing::warn!(stop = stop_id, descartados = ignored, "[MVD/upcomingbuses] ignored filter");
    }

    if failed == total && total > 0 {
      return Err(crate::core::Error::External(format!("upcomingbuses failed for stop {stop_id}")));
    }

    tracing::info!(
      stop = stop_id,
      lines = lines.len(),
      chunks = total,
      failed = failed,
      buses = buses,
      "[MVD/upcomingbuses] OK"
    );
    Ok(by_line)
  }
}

//# GET <MVD>/buses?lines={lines}
// - realtime location & ETA
pub type ImLines = HashMap<String, Vec<StmLineBus>>;
impl StmApi {
  pub async fn lines(&self, lines: &[String]) -> crate::core::Result<ImLines> {
    let url = format!("{}/buses?lines={}", self.url, lines.join(","));
    let token = self.auth.token(&self.http).await?;

    let res = self.http.get(&url).bearer_auth(&token).send().await?;
    let status = res.status();

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
      tracing::warn!(url = %url, "[MVD/buses] rate limited (429)");
      return Err(crate::core::Error::RateLimited);
    }

    if !status.is_success() {
      let body = res.text().await.unwrap_or_default();
      tracing::warn!(status = status.as_u16(), url = %url, body = %body, "[MVD/buses] non-2xx");
      return Ok(ImLines::new());
    }

    let bytes = res.bytes().await.map_err(|e| {
      tracing::error!(url = %url, error = %e, "[MVD/buses] failed to read response body");
      crate::core::Error::External(format!("body read error at {url}: {e}"))
    })?;

    let flat: Vec<StmLineBus> = serde_json::from_slice(&bytes).map_err(|e| {
      let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
      tracing::error!(url = %url, error = %e, body = %snippet, "[MVD/buses] JSON decode failed");
      crate::core::Error::External(format!("JSON decode at {url}: {e}"))
    })?;

    tracing::info!(lines = lines.len(), buses = flat.len(), "[MVD/buses] OK");

    let requested: HashSet<&str> = lines.iter().map(String::as_str).collect();

    let mut by_line = ImLines::new();
    let mut ignored = 0;
    for bus in flat {
      if requested.contains(bus.bus.line.as_str()) {
        by_line.entry(bus.bus.line.clone()).or_default().push(bus);
      } else {
        ignored += 1;
      }
    }

    if ignored > 0 {
      tracing::warn!(descartados = ignored, "[MVD/upcomingbuses] ignored filter");
    }

    Ok(by_line)
  }
}

//# GET <MVD>/buses/gtfs/static/latest/...
// - GTFS static data
//- https://gtfs.org/
impl StmApi {
  const GTFS_URL: &str = "/buses/gtfs/static/latest";

  pub async fn gtfs_version(&self) -> crate::core::Result<String> {
    let token = self.auth.token(&self.http).await?;
    let url = format!("{}{}/version.txt", self.url, Self::GTFS_URL);

    let res = self
      .http
      .get(&url)
      .bearer_auth(&token)
      .send()
      .await
      .map_err(|e| crate::core::Error::External(format!("GTFS version request failed: {e}")))?;

    if !res.status().is_success() {
      return Err(crate::core::Error::External(format!("GTFS version returned HTTP {}", res.status())));
    }

    let version =
      res.text().await.map_err(|e| crate::core::Error::External(format!("GTFS version read failed: {e}")))?;
    Ok(version.trim().to_string())
  }

  pub async fn gtfs_download(&self) -> crate::core::Result<Vec<u8>> {
    let token = self.auth.token(&self.http).await?;
    let url = format!("{}{}/google_transit.zip", self.url, Self::GTFS_URL);

    let res = self
      .http
      .get(&url)
      .bearer_auth(&token)
      .send()
      .await
      .map_err(|e| crate::core::Error::External(format!("GTFS ZIP request failed: {e}")))?;

    if !res.status().is_success() {
      return Err(crate::core::Error::External(format!("GTFS ZIP download returned HTTP {}", res.status())));
    }

    res.bytes().await.map(|b| b.to_vec()).map_err(|e| crate::core::Error::External(format!("ZIP read failed: {e}")))
  }
}

//# GET <MVD>/buses?lines={test_line}
// - realtime API health
impl StmApi {
  pub async fn probe(&self) -> bool {
    let Ok(token) = self.auth.token(&self.http).await else {
      return false;
    };
    let url = format!("{}/buses?lines=151", self.url);
    match self.http.get(&url).bearer_auth(token).send().await {
      Ok(resp) => resp.status().is_success() || resp.status().as_u16() < 500,
      _ => false,
    }
  }
}
