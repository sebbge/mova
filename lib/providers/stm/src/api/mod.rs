// https://api.montevideo.gub.uy/apidocs/publictransport
mod auth;
pub mod models;
pub mod online;
use self::auth::StmAuth;
use self::models::{StmLineBus, StmUpcomingBus};
use crate::sdk::limiter::{Limiter, Priority};
use crate::sdk::{Error, Result, http};

use reqwest::{RequestBuilder, Response, StatusCode};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

fn decode_each<T: serde::de::DeserializeOwned>(bytes: &[u8], url: &str, tag: &str) -> Result<Vec<T>> {
  let raw: Vec<serde_json::Value> = serde_json::from_slice(bytes).map_err(|e| {
    let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
    tracing::error!(url = %url, error = %e, body = %snippet, "{tag} JSON decode failed");
    Error::External(format!("JSON decode at {url}: {e}"))
  })?;

  let total = raw.len();
  let mut first_error = None;
  let items: Vec<T> = raw
    .into_iter()
    .filter_map(|item| match T::deserialize(&item) {
      Ok(parsed) => Some(parsed),
      Err(e) => {
        first_error.get_or_insert_with(|| format!("{e}: {item}"));
        None
      }
    })
    .collect();

  if let Some(error) = first_error {
    tracing::warn!(url = %url, dropped = total - items.len(), error = %error, "{tag} malformed buses dropped");
  }
  Ok(items)
}

fn by_line<T>(items: Vec<T>, lines: &[String], line: impl Fn(&T) -> &str, tag: &str) -> HashMap<String, Vec<T>> {
  let requested: HashSet<&str> = lines.iter().map(String::as_str).collect();

  let mut by_line: HashMap<String, Vec<T>> = HashMap::new();
  let mut dropped = 0;
  for item in items {
    let key = line(&item).to_owned();
    if requested.contains(key.as_str()) {
      by_line.entry(key).or_default().push(item);
    } else {
      dropped += 1;
    }
  }

  if dropped > 0 {
    tracing::warn!(dropped, "{tag} line filter ignored");
  }
  by_line
}

pub struct StmApi {
  http: reqwest::Client,
  url: String,
  auth: StmAuth,
  limiter: Limiter,
  upcoming: Mutex<HashMap<String, (Instant, UpcomingCell)>>,
  rejected: Mutex<HashMap<(String, String), Instant>>,
}
impl StmApi {
  const CAPACITY: u32 = 6;
  const REFILL: Duration = Duration::from_secs(5);
  const RETRY_AFTER: Duration = Duration::from_secs(30);

  pub fn new(url: String, auth_url: String, client_id: String, client_secret: String) -> Result<Self> {
    let http = http::client(Duration::from_secs(12))?;
    let auth = StmAuth::new(auth_url, client_id, client_secret);
    let limiter = Limiter::new(Self::CAPACITY, Self::REFILL);
    Ok(Self { http, url, auth, limiter, upcoming: Mutex::default(), rejected: Mutex::default() })
  }

  async fn send(&self, req: RequestBuilder, tag: &str, priority: Priority) -> Result<Response> {
    if let Err(e) = self.limiter.acquire(priority).await {
      tracing::debug!(source = tag, "[stm/limit] request throttled");
      return Err(e);
    }
    let res = http::send(req, tag).await?;
    if res.status() == StatusCode::TOO_MANY_REQUESTS {
      let pause = http::retry_after(&res).unwrap_or(Self::RETRY_AFTER);
      tracing::debug!(source = tag, secs = pause.as_secs(), "[stm/limit] paused");
      self.limiter.penalize(pause);
    }
    Ok(res)
  }

  async fn get(&self, url: &str, tag: &str, priority: Priority) -> Result<Response> {
    let token = self.auth.token(&self.http).await?;
    self.send(self.http.get(url).bearer_auth(&token), tag, priority).await
  }
}

//# GET <MVD>/buses/busstops/{stop_id}/upcomingbuses?lines={lines}&amountperline=3
// - upcoming buses

pub type StmUpcomingLines = HashMap<String, Vec<StmUpcomingBus>>;
type UpcomingCell = Arc<OnceCell<StmUpcomingLines>>;
impl StmApi {
  const REJECTED_TTL: Duration = Duration::from_secs(3600);
  const UPCOMING_TTL: Duration = Duration::from_secs(20);
  const UPCOMING_CHUNK: usize = 8;
  const UPCOMING_CONCURRENCY: usize = 3;

  async fn fetch_upcoming(&self, stop_id: &str, lines: &[String]) -> Result<Option<Vec<StmUpcomingBus>>> {
    let url =
      format!("{}/buses/busstops/{}/upcomingbuses?lines={}&amountperline=3", self.url, stop_id, lines.join(","));

    let res = self.get(&url, "[stm/upcoming]", Priority::Interactive).await?;
    if res.status() == StatusCode::BAD_REQUEST {
      return Ok(None);
    }
    let res = http::ok(res, "[stm/upcoming]").await?;

    let bytes = http::bytes(res, "[stm/upcoming]").await?;
    let mut upcoming: Vec<StmUpcomingBus> = decode_each(&bytes, &url, "[stm/upcoming]")?;
    if let [line] = lines {
      for u in upcoming.iter_mut().filter(|u| u.bus.line.is_empty()) {
        u.bus.line.clone_from(line);
      }
    }
    Ok(Some(upcoming))
  }

  async fn upcoming_chunk(&self, stop_id: &str, lines: &[String]) -> Result<Vec<StmUpcomingBus>> {
    if let Some(upcoming) = self.fetch_upcoming(stop_id, lines).await? {
      return Ok(upcoming);
    }

    let [line] = lines else {
      let (left, right) = lines.split_at(lines.len() / 2);
      let mut upcoming = Box::pin(self.upcoming_chunk(stop_id, left)).await?;
      upcoming.extend(Box::pin(self.upcoming_chunk(stop_id, right)).await?);
      return Ok(upcoming);
    };

    tracing::warn!(stop = stop_id, line = %line, "[stm/upcoming] line rejected (400)");
    if let Ok(mut rejected) = self.rejected.lock() {
      rejected.insert((stop_id.to_owned(), line.clone()), Instant::now());
    }
    Ok(Vec::new())
  }

  fn accepted(&self, stop_id: &str, lines: &[String]) -> Vec<String> {
    let Ok(mut rejected) = self.rejected.lock() else {
      return lines.to_vec();
    };
    rejected.retain(|_, at| at.elapsed() < Self::REJECTED_TTL);
    lines.iter().filter(|line| !rejected.contains_key(&(stop_id.to_owned(), (*line).clone()))).cloned().collect()
  }

  async fn load_upcoming(&self, stop_id: &str, lines: &[String]) -> Result<StmUpcomingLines> {
    use futures_util::StreamExt;

    let accepted = self.accepted(stop_id, lines);
    let pending: Vec<_> =
      accepted.chunks(Self::UPCOMING_CHUNK).map(|chunk| self.upcoming_chunk(stop_id, chunk)).collect();
    let chunks = pending.len();

    let results: Vec<_> =
      futures_util::stream::iter(pending).buffer_unordered(Self::UPCOMING_CONCURRENCY).collect().await;

    let mut upcoming = Vec::new();
    let mut failed = 0;
    for result in results {
      match result {
        Ok(buses) => upcoming.extend(buses),
        Err(_) => failed += 1,
      }
    }

    if failed == chunks && chunks > 0 {
      return Err(Error::External(format!("upcomingbuses failed for stop {stop_id}")));
    }

    tracing::info!(stop = stop_id, lines = lines.len(), chunks, failed, buses = upcoming.len(), "[stm/upcoming] OK");
    Ok(by_line(upcoming, lines, |u| &u.bus.line, "[stm/upcoming]"))
  }

  fn upcoming_cell(&self, stop_id: &str) -> UpcomingCell {
    let Ok(mut cells) = self.upcoming.lock() else {
      return Arc::default();
    };
    cells.retain(|_, (at, _)| at.elapsed() < Self::UPCOMING_TTL);
    cells.entry(stop_id.to_owned()).or_insert_with(|| (Instant::now(), Arc::default())).1.clone()
  }

  fn forget_upcoming(&self, stop_id: &str, cell: &UpcomingCell) {
    if let Ok(mut cells) = self.upcoming.lock()
      && cells.get(stop_id).is_some_and(|(_, c)| Arc::ptr_eq(c, cell))
    {
      cells.remove(stop_id);
    }
  }

  pub async fn upcoming_lines(&self, stop_id: &str, lines: &[String]) -> Result<StmUpcomingLines> {
    let cell = self.upcoming_cell(stop_id);
    let upcoming = cell.get_or_try_init(|| self.load_upcoming(stop_id, lines)).await.cloned();
    if upcoming.is_err() {
      self.forget_upcoming(stop_id, &cell);
    }
    upcoming
  }
}

//# GET <MVD>/buses
// - whole fleet for realtime location
impl StmApi {
  pub async fn fleet(&self) -> Result<Vec<StmLineBus>> {
    let url = format!("{}/buses", self.url);
    let res = http::ok(self.get(&url, "[stm/buses]", Priority::Background).await?, "[stm/buses]").await?;
    let bytes = http::bytes(res, "[stm/buses]").await?;
    let fleet: Vec<StmLineBus> = decode_each(&bytes, &url, "[stm/buses]")?;

    tracing::info!(buses = fleet.len(), "[stm/buses] OK");
    Ok(fleet)
  }

  pub async fn online(&self) -> Result<Vec<online::OnlineBus>> {
    online::buses(&self.http).await
  }
}

//# GET <MVD>/buses/gtfs/static/latest/...
// - GTFS static data
//- https://gtfs.org/
impl StmApi {
  const GTFS_URL: &str = "/buses/gtfs/static/latest";

  pub async fn gtfs_version(&self) -> Result<String> {
    let url = format!("{}{}/version.txt", self.url, Self::GTFS_URL);
    let res = http::ok(self.get(&url, "[stm/gtfs]", Priority::Background).await?, "[stm/gtfs]").await?;
    let version = res.text().await?;
    Ok(version.trim().to_string())
  }

  pub async fn gtfs_download(&self) -> Result<Vec<u8>> {
    let url = format!("{}{}/google_transit.zip", self.url, Self::GTFS_URL);
    let res = http::ok(self.get(&url, "[stm/gtfs]", Priority::Background).await?, "[stm/gtfs]").await?;
    http::bytes(res, "[stm/gtfs]").await
  }
}
