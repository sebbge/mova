mod bus_tracker;
mod encoding;
pub use bus_tracker::{BusTracker, StopGhost, Tracked, TrailPoint};
pub use encoding::Encoding;

use redis::AsyncCommands;

type Conn = redis::aio::MultiplexedConnection;
pub(super) struct Cache {
  conn: Conn,
  sync: i64,
  bus_tracker: BusTracker,
}
impl Cache {
  /// Region prefix
  pub const SHARED: &str = "all";

  pub(super) fn new(conn: Conn, sync: i64) -> Self {
    Self { conn, sync, bus_tracker: BusTracker::default() }
  }

  pub fn until_sync(&self) -> u64 {
    use chrono::{Local, Timelike};
    let now = Local::now().time().num_seconds_from_midnight() as i64;
    let remaining = if self.sync > now { self.sync - now } else { 86400 - now + self.sync };
    remaining.max(60) as u64
  }
}

impl Cache {
  pub fn key(region: &str, rest: &str) -> String {
    format!("{region}:{rest}")
  }

  pub fn until_midnight() -> u64 {
    use chrono::{Local, Timelike};
    let now = Local::now();
    (86_400 - now.num_seconds_from_midnight() as u64).max(1)
  }

  pub async fn get_or_load<T, F>(&self, key: &str, ttl: u64, load: F) -> crate::Result<T>
  where
    T: serde::Serialize + serde::de::DeserializeOwned,
    F: Future<Output = crate::Result<T>>,
  {
    if let Some(hit) = self.get(key).await {
      return Ok(hit);
    }
    let value = load.await?;
    self.set(key, &value, ttl).await;
    Ok(value)
  }

  pub async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
    let mut conn = self.conn.clone();
    conn.get::<_, String>(key).await.ok().and_then(|s| serde_json::from_str(&s).ok())
  }

  pub async fn set<T: serde::Serialize>(&self, key: &str, val: &T, ttl: u64) {
    if let Ok(json) = serde_json::to_string(val) {
      let mut conn = self.conn.clone();
      let _: Result<(), _> = conn.set_ex(key, json, ttl).await;
    }
  }

  /// Precompressed bodies stay bytes
  pub async fn get_bytes(&self, key: &str) -> Option<Vec<u8>> {
    let mut conn = self.conn.clone();
    conn.get::<_, Vec<u8>>(key).await.ok().filter(|b| !b.is_empty())
  }

  pub async fn set_bytes(&self, key: &str, val: &[u8], ttl: u64) {
    let mut conn = self.conn.clone();
    let _: Result<(), _> = conn.set_ex(key, val, ttl).await;
  }

  pub fn tracker(&self) -> &BusTracker {
    &self.bus_tracker
  }

  //# after a feed sync
  pub async fn flush_region(&self, region: crate::sdk::region::Region) {
    for prefix in [region.id(), Self::SHARED] {
      if let Err(e) = self.drop_prefix(&format!("{prefix}:")).await {
        tracing::warn!(prefix, error = %e, "[cache] couldn't flush region")
      }
    }
  }

  /// `SCAN`, not the blocking `KEYS`
  async fn drop_prefix(&self, prefix: &str) -> redis::RedisResult<usize> {
    let mut conn = self.conn.clone();
    let pattern = format!("{prefix}*");
    let mut cursor: u64 = 0;
    let mut removed = 0usize;

    loop {
      let (next, keys): (u64, Vec<String>) =
        redis::cmd("SCAN").arg(cursor).arg("MATCH").arg(&pattern).arg("COUNT").arg(500).query_async(&mut conn).await?;

      if !keys.is_empty() {
        removed += keys.len();
        redis::cmd("DEL").arg(&keys).query_async::<()>(&mut conn).await?;
      }

      cursor = next;
      if cursor == 0 {
        return Ok(removed);
      }
    }
  }
}
