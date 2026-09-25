use crate::{Error, Result};

use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Clone, Copy, Debug)]
pub enum Priority {
  Background,
  Interactive,
}
impl Priority {
  fn max_wait(self) -> Duration {
    match self {
      Self::Background => Duration::from_secs(15),
      Self::Interactive => Duration::from_secs(1),
    }
  }

  fn reserve(self) -> f64 {
    match self {
      Self::Background => 0.0,
      Self::Interactive => 1.0,
    }
  }
}

struct Bucket {
  tokens: f64,
  last: Instant,
}

pub struct Limiter {
  capacity: f64,
  refill: Duration,
  bucket: Mutex<Bucket>,
}
impl Limiter {
  const MAX_PENALTY: Duration = Duration::from_secs(60);

  pub fn new(capacity: u32, refill: Duration) -> Self {
    let capacity = f64::from(capacity);
    Self { capacity, refill, bucket: Mutex::new(Bucket { tokens: capacity, last: Instant::now() }) }
  }

  pub async fn acquire(&self, priority: Priority) -> Result<()> {
    let wait = self.reserve(priority).ok_or(Error::RateLimited)?;
    if !wait.is_zero() {
      tokio::time::sleep(wait).await;
    }
    Ok(())
  }

  pub fn penalize(&self, retry_after: Duration) {
    let mut bucket = self.bucket.lock().unwrap_or_else(|e| e.into_inner());
    bucket.tokens = 0.0;
    bucket.last = bucket.last.max(Instant::now() + retry_after.min(Self::MAX_PENALTY));
  }

  fn reserve(&self, priority: Priority) -> Option<Duration> {
    let mut bucket = self.bucket.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let refill = self.refill.as_secs_f64();

    if now > bucket.last {
      bucket.tokens = (bucket.tokens + (now - bucket.last).as_secs_f64() / refill).min(self.capacity);
      bucket.last = now;
    }

    let missing = (1.0 + priority.reserve() - bucket.tokens).max(0.0);
    let wait = bucket.last.saturating_duration_since(now) + Duration::from_secs_f64(missing * refill);
    if wait > priority.max_wait() {
      return None;
    }
    bucket.tokens -= 1.0;
    Some(wait)
  }
}

#[cfg(test)]
mod tests {
  use super::{Limiter, Priority};

  use std::time::Duration;

  const REFILL: Duration = Duration::from_secs(5);

  #[tokio::test(start_paused = true)]
  async fn burst_then_refill() {
    let limiter = Limiter::new(3, REFILL);
    for _ in 0..3 {
      assert_eq!(limiter.reserve(Priority::Background), Some(Duration::ZERO));
    }
    assert_eq!(limiter.reserve(Priority::Background), Some(REFILL));

    tokio::time::advance(REFILL * 2).await;
    assert_eq!(limiter.reserve(Priority::Background), Some(Duration::ZERO));
  }

  #[tokio::test(start_paused = true)]
  async fn interactive_leaves_one_for_background() {
    let limiter = Limiter::new(2, REFILL);
    assert_eq!(limiter.reserve(Priority::Interactive), Some(Duration::ZERO));
    assert_eq!(limiter.reserve(Priority::Interactive), None);
    assert_eq!(limiter.reserve(Priority::Background), Some(Duration::ZERO));
  }

  #[tokio::test(start_paused = true)]
  async fn penalty_blocks_until_retry_after() {
    let limiter = Limiter::new(6, REFILL);
    limiter.penalize(Duration::from_secs(30));
    assert_eq!(limiter.reserve(Priority::Background), None);
    assert_eq!(limiter.reserve(Priority::Interactive), None);

    tokio::time::advance(Duration::from_secs(30)).await;
    assert_eq!(limiter.reserve(Priority::Background), Some(REFILL));
  }
}
