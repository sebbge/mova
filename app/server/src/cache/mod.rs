use redis::AsyncCommands;

mod bus_tracker;
pub use bus_tracker::{BusTracker, StopGhost};

type Conn = redis::aio::MultiplexedConnection;
pub(super) struct Cache {
  conn: Conn,
  sync: i64,
  bus_tracker: BusTracker,
}
impl Cache {
  pub(super) fn new(conn: Conn, sync: i64) -> Self {
    Self { conn, sync, bus_tracker: BusTracker::new() }
  }

  pub fn until_sync(&self) -> u64 {
    use chrono::{Local, Timelike};
    let now = Local::now().time().num_seconds_from_midnight() as i64;
    let remaining = if self.sync > now { self.sync - now } else { 86400 - now + self.sync };
    remaining.max(60) as u64
  }
}

pub trait CacheRw {
  async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T>;
  async fn set<T: serde::Serialize>(&self, key: &str, val: &T, ttl: u64);
}
impl CacheRw for Cache {
  async fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
    let mut conn = self.conn.clone();
    conn.get::<_, String>(key).await.ok().and_then(|s| serde_json::from_str(&s).ok())
  }

  async fn set<T: serde::Serialize>(&self, key: &str, val: &T, ttl: u64) {
    if let Ok(json) = serde_json::to_string(val) {
      let mut conn = self.conn.clone();
      let _: Result<(), _> = conn.set_ex(key, json, ttl).await;
    }
  }
}

pub trait BusTracking {
  fn tracker(&self) -> &BusTracker;
}
impl BusTracking for Cache {
  fn tracker(&self) -> &BusTracker {
    &self.bus_tracker
  }
}

//# FLUSHDB after GTFS sync
pub trait Flush {
  async fn flush(&self);
}

impl Flush for Cache {
  async fn flush(&self) {
    let mut conn = self.conn.clone();
    match redis::cmd("FLUSHDB").query_async::<()>(&mut conn).await {
      Ok(_) => tracing::info!("Redis caches flushed"),
      Err(e) => tracing::warn!(error = %e, "failed to flush Redis"),
    }
  }
}
