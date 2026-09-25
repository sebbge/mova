#![allow(clippy::expect_used, clippy::unwrap_used)]

use citymob_core::provider::Vehicles;
use citymob_stm::{StmConfig, provider};

use std::collections::HashSet;
use std::time::{Duration, Instant};

const STOP_ID: &str = "3663";
const LINES: [&str; 7] = ["169", "128", "370", "151", "522", "G3", "175"];

fn config() -> StmConfig {
  dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.env")).ok();
  let var = |key: &str| std::env::var(key).expect(key);
  StmConfig {
    api_url: var("IM_API_URL"),
    auth_url: var("IM_AUTH_URL"),
    client_id: var("IM_CLIENT_ID"),
    client_secret: var("IM_CLIENT_SECRET"),
  }
}

#[tokio::test]
#[ignore = "network: hits the IM"]
async fn one_request_brings_the_whole_fleet() {
  let stm = provider(config()).unwrap();
  let Some(Vehicles::Fleet(fleet)) = stm.vehicles.clone() else { panic!("stm should declare Fleet") };

  let buses = fleet.fleet().await.expect("fleet");
  let ids: HashSet<&str> = buses.iter().map(|b| b.id.as_str()).collect();
  assert!(buses.len() > 100, "the whole fleet should come back: {}", buses.len());
  assert!(ids.len() * 100 >= buses.len() * 99, "the physical id is company + unit");
  assert!(buses.iter().all(|b| b.id == format!("{}:{}", b.company, b.unit)));
}

#[tokio::test]
#[ignore = "network: hits the IM"]
async fn concurrent_arrivals_share_one_request() {
  let stm = provider(config()).unwrap();
  let arrivals = stm.arrivals.clone().unwrap();
  let lines: Vec<String> = LINES.iter().map(|l| (*l).to_owned()).collect();

  let started = Instant::now();
  let upcoming = futures_util::future::join_all((0..5).map(|_| arrivals.upcoming(STOP_ID, &lines))).await;
  let first = upcoming[0].as_ref().expect("upcomingbuses");
  for other in &upcoming[1..] {
    let other = other.as_ref().expect("upcomingbuses");
    assert_eq!(first.len(), other.len(), "all 5 should share a single request");
  }

  let again = Instant::now();
  assert!(arrivals.upcoming(STOP_ID, &lines).await.is_ok());
  assert!(again.elapsed() < Duration::from_millis(50), "within the TTL it should not request again");
  assert!(started.elapsed() < Duration::from_secs(15));
}
