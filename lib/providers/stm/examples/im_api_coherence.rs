//! cargo run --example im_api_coherence (IM_CLIENT_ID + IM_CLIENT_SECRET in .env)
use std::collections::{HashMap, HashSet};

// Montevideo bbox
const LAT_MIN: f64 = -35.0;
const LAT_MAX: f64 = -34.5;
const LNG_MIN: f64 = -56.5;
const LNG_MAX: f64 = -55.8;

const STOP_ID: &str = "3663"; // CORUÑA y PURIFICACION
const SAMPLE_LINES: &str = "169,128,370,151,522,G3,175";

const MAX_DRIFT_M: f64 = 500.0;

// = helpers

fn base_url() -> String {
  std::env::var("IM_API_URL").unwrap_or_else(|_| "https://api.montevideo.gub.uy/api/transportepublico".into())
}

fn haversine_m(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
  let r = 6_371_000.0_f64;
  let d_lat = (lat2 - lat1).to_radians();
  let d_lng = (lng2 - lng1).to_radians();
  let a = (d_lat / 2.0).sin().powi(2) + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
  r * 2.0 * a.sqrt().asin()
}

async fn auth(client: &reqwest::Client) -> String {
  let auth_url = std::env::var("IM_AUTH_URL")
    .unwrap_or_else(|_| "https://mvdapi-auth.montevideo.gub.uy/auth/realms/pci/protocol/openid-connect/token".into());
  let client_id = std::env::var("IM_CLIENT_ID").expect("IM_CLIENT_ID");
  let client_secret = std::env::var("IM_CLIENT_SECRET").expect("IM_CLIENT_SECRET");

  let res: serde_json::Value = client
    .post(&auth_url)
    .form(&[
      ("grant_type", "client_credentials"),
      ("client_id", client_id.as_str()),
      ("client_secret", client_secret.as_str()),
    ])
    .send()
    .await
    .expect("auth request failed")
    .json()
    .await
    .expect("auth json failed");

  res["access_token"].as_str().expect("missing access_token").to_string()
}

fn assert_montevideo(lat: f64, lng: f64, ctx: &str) {
  assert!(
    (LAT_MIN..=LAT_MAX).contains(&lat) && (LNG_MIN..=LNG_MAX).contains(&lng),
    "{ctx}: coords ({lat}, {lng}) outside Montevideo bounds"
  );
}

// = /buses

async fn buses_response_is_coherent() {
  dotenvy::dotenv().ok();

  let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().unwrap();
  let base = base_url();
  let token = auth(&client).await;

  let url = format!("{base}/buses?lines={SAMPLE_LINES}");
  let res = client.get(&url).bearer_auth(&token).send().await.expect("request failed");
  assert!(res.status().is_success(), "/buses returned {}", res.status());

  let buses: Vec<serde_json::Value> = res.json().await.expect("json parse failed");
  eprintln!("  /buses returned {} buses", buses.len());

  let requested: HashSet<&str> = SAMPLE_LINES.split(',').collect();

  for (i, bus) in buses.iter().enumerate() {
    let ctx = format!("bus[{i}]");

    // bus_id
    let bus_id = bus["busId"].as_u64().unwrap_or(0);
    assert!(bus_id > 0, "{ctx}: busId missing or zero");

    // line matches request
    let line = bus["line"].as_str().unwrap_or("");
    assert!(!line.is_empty(), "{ctx}: line is empty");
    assert!(requested.contains(line), "{ctx}: line '{line}' not in requested lines");

    // [lng, lat]
    let coords = bus["location"]["coordinates"].as_array();
    assert!(coords.is_some(), "{ctx}: missing location.coordinates");
    let coords = coords.unwrap();
    assert_eq!(coords.len(), 2, "{ctx}: coordinates should have exactly 2 elements, got {}", coords.len());

    let lng = coords[0].as_f64().unwrap_or_else(|| panic!("{ctx}: lng not a number"));
    let lat = coords[1].as_f64().unwrap_or_else(|| panic!("{ctx}: lat not a number"));
    assert_montevideo(lat, lng, &ctx);

    // origin / destination not empty
    let origin = bus["origin"].as_str().unwrap_or("");
    let destination = bus["destination"].as_str().unwrap_or("");
    assert!(!origin.is_empty(), "{ctx}: origin is empty (bus_id={bus_id}, line={line})");
    assert!(!destination.is_empty(), "{ctx}: destination is empty (bus_id={bus_id}, line={line})");

    // company
    let company = bus["company"].as_str().unwrap_or("");
    assert!(!company.is_empty(), "{ctx}: company is empty");
  }

  eprintln!("  ✅ all {} buses passed coherence checks", buses.len());
}

// = /upcomingbuses

async fn upcomingbuses_response_is_coherent() {
  dotenvy::dotenv().ok();

  let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().unwrap();
  let base = base_url();
  let token = auth(&client).await;

  let lines_url = format!("{base}/buses/busstops/{STOP_ID}/lines");
  let lines_res = client.get(&lines_url).bearer_auth(&token).send().await.expect("lines request failed");
  let stop_lines: Vec<serde_json::Value> =
    if lines_res.status().is_success() { lines_res.json().await.unwrap_or_default() } else { vec![] };
  let known_lines: HashSet<String> =
    stop_lines.iter().filter_map(|v| v["lineNumber"].as_str().map(|s| s.to_string())).collect();

  if !known_lines.is_empty() {
    eprintln!("  stop {STOP_ID} is served by {} lines: {:?}", known_lines.len(), known_lines);
  }

  let url = format!("{base}/buses/busstops/{STOP_ID}/upcomingbuses?lines={SAMPLE_LINES}&amountperline=3");
  let res = client.get(&url).bearer_auth(&token).send().await.expect("request failed");
  assert!(res.status().is_success(), "/upcomingbuses returned {}", res.status());

  let upcoming: Vec<serde_json::Value> = res.json().await.expect("json parse failed");
  eprintln!("  /upcomingbuses returned {} entries", upcoming.len());

  let requested: HashSet<&str> = SAMPLE_LINES.split(',').collect();

  for (i, bus) in upcoming.iter().enumerate() {
    let ctx = format!("upcoming[{i}]");

    // bus_id
    let bus_id = bus["busId"].as_u64().unwrap_or(0);
    assert!(bus_id > 0, "{ctx}: busId missing or zero");

    // line matches request
    let line = bus["line"].as_str().unwrap_or("");
    assert!(!line.is_empty(), "{ctx}: line is empty");
    assert!(requested.contains(line), "{ctx}: line '{line}' not in requested lines");

    // line serves the stop
    if !known_lines.is_empty() {
      assert!(
        known_lines.contains(line),
        "{ctx}: line '{line}' returned in upcoming but doesn't serve stop {STOP_ID} per /lines endpoint"
      );
    }

    // coordinates
    let coords = bus["location"]["coordinates"].as_array();
    assert!(coords.is_some(), "{ctx}: missing location.coordinates");
    let coords = coords.unwrap();
    assert_eq!(coords.len(), 2, "{ctx}: coordinates should have 2 elements");

    let lng = coords[0].as_f64().unwrap_or_else(|| panic!("{ctx}: lng not a number"));
    let lat = coords[1].as_f64().unwrap_or_else(|| panic!("{ctx}: lat not a number"));
    assert_montevideo(lat, lng, &ctx);

    // origin / destination
    let origin = bus["origin"].as_str().unwrap_or("");
    let destination = bus["destination"].as_str().unwrap_or("");
    assert!(!origin.is_empty(), "{ctx}: origin is empty (bus_id={bus_id}, line={line})");
    assert!(!destination.is_empty(), "{ctx}: destination is empty (bus_id={bus_id}, line={line})");

    // eta (s) >= 0
    if let Some(eta) = bus.get("eta").and_then(|v| v.as_i64()) {
      assert!(eta >= 0, "{ctx}: eta is negative ({eta}s) — bus_id={bus_id}, line={line}");
    }

    // distance (m) >= 0
    if let Some(distance) = bus.get("distance").and_then(|v| v.as_i64()) {
      assert!(distance >= 0, "{ctx}: distance is negative ({distance}m) — bus_id={bus_id}, line={line}");
    }
  }

  // amountperline=3
  let mut per_line: HashMap<&str, usize> = HashMap::new();
  for bus in &upcoming {
    if let Some(line) = bus["line"].as_str() {
      *per_line.entry(line).or_default() += 1;
    }
  }
  for (line, count) in &per_line {
    assert!(*count <= 3, "line '{line}' returned {count} upcoming buses, expected ≤ 3 (amountperline=3)");
  }

  eprintln!("  ✅ all {} upcoming buses passed coherence checks", upcoming.len());
}

// = cross-endpoint: /buses × /upcomingbuses

// `company` in /buses, `companyName` in /upcomingbuses
async fn cross_endpoint_coherence() {
  dotenvy::dotenv().ok();

  let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build().unwrap();
  let base = base_url();
  let token = auth(&client).await;

  // /buses
  let buses_url = format!("{base}/buses?lines={SAMPLE_LINES}");
  let buses_res = client.get(&buses_url).bearer_auth(&token).send().await.expect("/buses request failed");
  assert!(buses_res.status().is_success(), "/buses returned {}", buses_res.status());
  let buses: Vec<serde_json::Value> = buses_res.json().await.expect("/buses json failed");

  // /upcomingbuses
  let upcoming_url = format!("{base}/buses/busstops/{STOP_ID}/upcomingbuses?lines={SAMPLE_LINES}&amountperline=10");
  let upcoming_res = client.get(&upcoming_url).bearer_auth(&token).send().await.expect("/upcomingbuses request failed");
  assert!(upcoming_res.status().is_success(), "/upcomingbuses returned {}", upcoming_res.status());
  let upcoming: Vec<serde_json::Value> = upcoming_res.json().await.expect("/upcomingbuses json failed");

  eprintln!("  /buses: {} buses — /upcomingbuses: {} entries", buses.len(), upcoming.len());

  let buses_by_id: HashMap<u64, &serde_json::Value> =
    buses.iter().filter_map(|b| b["busId"].as_u64().map(|id| (id, b))).collect();

  let mut matched = 0u32;
  let mut diffs = Vec::<String>::new();

  for ub in &upcoming {
    let bus_id = match ub["busId"].as_u64() {
      Some(id) => id,
      None => continue,
    };

    let b = match buses_by_id.get(&bus_id) {
      Some(b) => b,
      None => {
        // /upcomingbuses may hold buses filtered out of /buses
        continue;
      }
    };

    matched += 1;
    let ctx = format!("busId={bus_id}");

    // line
    let b_line = b["line"].as_str().unwrap_or("");
    let u_line = ub["line"].as_str().unwrap_or("");
    assert_eq!(b_line, u_line, "{ctx}: line differs — /buses='{b_line}' vs /upcoming='{u_line}'");

    // company
    let b_company = b["company"].as_str().unwrap_or("");
    let u_company = ub["companyName"].as_str().unwrap_or("");
    if !b_company.is_empty() && !u_company.is_empty() && !b_company.eq_ignore_ascii_case(u_company) {
      diffs.push(format!("{ctx}: company — /buses='{b_company}' vs /upcoming='{u_company}'"));
    }

    // origin
    let b_origin = b["origin"].as_str().unwrap_or("");
    let u_origin = ub["origin"].as_str().unwrap_or("");
    if !b_origin.is_empty() && !u_origin.is_empty() && !b_origin.eq_ignore_ascii_case(u_origin) {
      diffs.push(format!("{ctx}: origin — /buses='{b_origin}' vs /upcoming='{u_origin}'"));
    }

    // destination
    let b_dest = b["destination"].as_str().unwrap_or("");
    let u_dest = ub["destination"].as_str().unwrap_or("");
    if !b_dest.is_empty() && !u_dest.is_empty() && !b_dest.eq_ignore_ascii_case(u_dest) {
      diffs.push(format!("{ctx}: destination — /buses='{b_dest}' vs /upcoming='{u_dest}'"));
    }

    // subline
    let b_sub = b["subline"].as_str().unwrap_or("");
    let u_sub = ub["subline"].as_str().unwrap_or("");
    if !b_sub.is_empty() && !u_sub.is_empty() && !b_sub.eq_ignore_ascii_case(u_sub) {
      diffs.push(format!("{ctx}: subline — /buses='{b_sub}' vs /upcoming='{u_sub}'"));
    }

    // lineVariantId
    let b_lv = b["lineVariantId"].as_u64();
    let u_lv = ub["lineVariantId"].as_u64();
    if let (Some(bv), Some(uv)) = (b_lv, u_lv)
      && bv != uv
    {
      diffs.push(format!("{ctx}: lineVariantId — /buses={bv} vs /upcoming={uv}"));
    }

    // location drift
    let b_coords = b["location"]["coordinates"].as_array();
    let u_coords = ub["location"]["coordinates"].as_array();
    if let (Some(bc), Some(uc)) = (b_coords, u_coords)
      && bc.len() == 2
      && uc.len() == 2
    {
      let b_lng = bc[0].as_f64().unwrap_or(0.0);
      let b_lat = bc[1].as_f64().unwrap_or(0.0);
      let u_lng = uc[0].as_f64().unwrap_or(0.0);
      let u_lat = uc[1].as_f64().unwrap_or(0.0);
      let drift = haversine_m(b_lat, b_lng, u_lat, u_lng);
      if drift > MAX_DRIFT_M {
        diffs.push(format!(
          "{ctx}: location drift {drift:.0}m (>{MAX_DRIFT_M}m) — \
             /buses=({b_lat},{b_lng}) vs /upcoming=({u_lat},{u_lng})"
        ));
      }
    }

    // special
    let b_special = b.get("special").and_then(|v| v.as_bool());
    let u_special = ub.get("special").and_then(|v| v.as_bool());
    if let (Some(bs), Some(us)) = (b_special, u_special)
      && bs != us
    {
      diffs.push(format!("{ctx}: special — /buses={bs} vs /upcoming={us}"));
    }
  }

  // Report
  eprintln!("  matched {matched} buses across both endpoints");

  if diffs.is_empty() {
    eprintln!("  ✅ all {matched} matched buses are cross-consistent");
  } else {
    eprintln!("  ⚠ {} field difference(s) found:", diffs.len());
    for d in &diffs {
      eprintln!("    · {d}");
    }
    panic!(
      "{} cross-endpoint difference(s) detected among {matched} matched buses:\n{}",
      diffs.len(),
      diffs.join("\n")
    );
  }

  assert!(matched > 0, "no buses appeared in both /buses and /upcomingbuses — can't cross-check");
}

#[tokio::main]
async fn main() {
  buses_response_is_coherent().await;
  upcomingbuses_response_is_coherent().await;
  cross_endpoint_coherence().await;
}
