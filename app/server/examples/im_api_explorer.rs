use std::fmt::Write;

const STOP_ID: &str = "3663";
const SAMPLE_LINE: &str = "169,128,370,151,522,G,175";

#[derive(PartialEq)]
enum Category {
  RealtimeOnly,
  GtfsReplaceable,
}

struct Hit {
  endpoint: &'static str,
  description: &'static str,
  gtfs_note: &'static str,
  category: Category,
  url: String,
  params: String,
  status: u16,
  body: String,
}

impl Hit {
  fn truncate_body(mut self, max_items: usize) -> Self {
    if let Ok(serde_json::Value::Array(mut arr)) = serde_json::from_str(&self.body) {
      let total = arr.len();
      if total > max_items {
        arr.truncate(max_items);
        let truncated =
          serde_json::to_string_pretty(&serde_json::Value::Array(arr)).unwrap_or_else(|_| self.body.clone());
        self.body = format!("{truncated}\n// ⚠ {total} total items — showing first {max_items}");
      }
    }
    self
  }
}

#[allow(clippy::too_many_arguments)]
async fn get(
  client: &reqwest::Client,
  token: &str,
  endpoint: &'static str,
  description: &'static str,
  gtfs_note: &'static str,
  category: Category,
  url: String,
  params: impl Into<String>,
) -> Hit {
  let params = params.into();
  eprintln!("  GET {url}");
  match client.get(&url).bearer_auth(token).send().await {
    Ok(r) => {
      let status = r.status().as_u16();
      let text = r.text().await.unwrap_or_default();
      let body = serde_json::from_str::<serde_json::Value>(&text)
        .map(|v| serde_json::to_string_pretty(&v).unwrap_or(text.clone()))
        .unwrap_or(text);
      Hit { endpoint, description, gtfs_note, category, url, params, status, body }
    }
    Err(e) => {
      Hit { endpoint, description, gtfs_note, category, url, params, status: 0, body: format!("REQUEST FAILED: {e}") }
    }
  }
}

fn render_hit(md: &mut String, r: &Hit) -> std::fmt::Result {
  let badge = match r.status {
    0 => "❌ ERROR".into(),
    200..=299 => format!("✅ {}", r.status),
    _ => format!("⚠️ {}", r.status),
  };

  writeln!(md, "### `{}`", r.endpoint)?;
  writeln!(md)?;
  writeln!(md, "{}", r.description)?;
  writeln!(md)?;
  writeln!(md, "| | |")?;
  writeln!(md, "|---|---|")?;
  writeln!(md, "| **Status** | {badge} |")?;
  writeln!(md, "| **URL** | `{}` |", r.url)?;
  writeln!(md, "| **Params** | `{}` |", r.params)?;
  writeln!(md, "| **GTFS note** | {} |", r.gtfs_note)?;
  writeln!(md)?;
  writeln!(md, "```json")?;
  writeln!(md, "{}", r.body)?;
  writeln!(md, "```")?;
  writeln!(md)?;

  Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  dotenvy::dotenv().ok();

  let base =
    std::env::var("IM_API_URL").unwrap_or_else(|_| "https://api.montevideo.gub.uy/api/transportepublico".into());
  let auth_url = std::env::var("IM_AUTH_URL")
    .unwrap_or_else(|_| "https://mvdapi-auth.montevideo.gub.uy/auth/realms/pci/protocol/openid-connect/token".into());
  let client_id = std::env::var("IM_CLIENT_ID").expect("IM_CLIENT_ID must be set");
  let client_secret = std::env::var("IM_CLIENT_SECRET").expect("IM_CLIENT_SECRET must be set");

  let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build()?;

  eprint!("→ authenticating... ");
  let auth: serde_json::Value = client
    .post(&auth_url)
    .form(&[
      ("grant_type", "client_credentials"),
      ("client_id", client_id.as_str()),
      ("client_secret", client_secret.as_str()),
    ])
    .send()
    .await?
    .json()
    .await?;
  let token = auth["access_token"].as_str().expect("missing access_token in auth response").to_string();
  eprintln!("OK");

  // Fetch /buses first to discover a real, active line.
  let line = SAMPLE_LINE;
  // let discovery_res = client.get(format!("{base}/buses")).bearer_auth(&token).send().await?;
  // let discovery_buses: Vec<serde_json::Value> = discovery_res.json().await.unwrap_or_default();
  // let line = discovery_buses.iter().filter_map(|b| b["line"].as_str()).next().unwrap_or("151").to_string();
  // eprintln!("→ discovered active line: {line}");

  let buses = get(
    &client,
    &token,
    "GET /buses",
    "Returns active buses in the system matching the given filters, including live GPS \
     coordinates, heading, company, and vehicle metadata (accessibility, emissions, A/C).",
    "No GTFS equivalent. GTFS has no real-time vehicle positions. Required for the \
     map layer showing bus icons and for computing haversine ETA when \
     `upcomingbuses` does not include GPS data.",
    Category::RealtimeOnly,
    format!("{base}/buses?lines={line}"),
    format!("lines={line}"),
  )
  .await;

  let upcomingbuses = get(
    &client,
    &token,
    "GET /buses/busstops/{busstopId}/upcomingbuses",
    "Returns upcoming buses at a stop with real-time ETAs and live GPS positions. \
     `lines` is required — without it the endpoint returns an empty array.",
    "No GTFS equivalent. GTFS `stop_times` gives scheduled departure times but \
     no live ETA or current vehicle position. This is the primary source for \
     `is_realtime: true` arrivals.",
    Category::RealtimeOnly,
    format!("{base}/buses/busstops/{STOP_ID}/upcomingbuses?lines={line}&amountperline=3"),
    format!("busstopId={STOP_ID}, lines={line}, amountperline=3"),
  )
  .await;

  //

  let linevariants = get(
    &client,
    &token,
    "GET /buses/linevariants",
    "Returns the complete set of line variants (route + direction + subline metadata).",
    "Fully covered by GTFS `routes.txt` + `trips.txt`. \
     `route_short_name` = line number, `trip_headsign` = destination, \
     `direction_id` = variant. No need to call this endpoint if the GTFS feed is loaded.",
    Category::GtfsReplaceable,
    format!("{base}/buses/linevariants"),
    "none",
  )
  .await;

  let first_variant_id = serde_json::from_str::<serde_json::Value>(&linevariants.body)
    .ok()
    .and_then(|v| v.as_array()?.first()?.get("lineVariantId")?.as_i64())
    .map(|id| id.to_string())
    .unwrap_or_else(|| "1234".into());

  let linevariant_detail = get(
    &client,
    &token,
    "GET /buses/linevariants/{lineVariantId}",
    "Returns detail for a single line variant by its internal numeric ID.",
    "Fully covered by GTFS. `lineVariantId` maps to a specific `route_id` + \
     `direction_id` pair in `routes.txt` / `trips.txt`. \
     Unnecessary when GTFS is the authoritative source.",
    Category::GtfsReplaceable,
    format!("{base}/buses/linevariants/{first_variant_id}"),
    format!("lineVariantId={first_variant_id} (first from /linevariants)"),
  )
  .await;

  let busstops = get(
    &client,
    &token,
    "GET /buses/busstops",
    "Returns all bus stops with name, corner streets, and GeoJSON coordinates.",
    "Fully covered by GTFS `stops.txt` (`stop_id`, `stop_name`, `stop_lat`, \
     `stop_lon`). The IM API adds `street1`/`street2` corner names, but those \
     are already embedded in `stop_name` in the GTFS feed. Large payload (~4 MB).",
    Category::GtfsReplaceable,
    format!("{base}/buses/busstops"),
    "none",
  )
  .await;

  let busstop_lines = get(
    &client,
    &token,
    "GET /buses/busstops/{busstopId}/lines",
    "Returns which lines serve a given stop.",
    "Fully covered by GTFS: `stop_times.txt` → `trips.txt` → `routes.txt` \
     gives the exact set of `route_short_name` values per `stop_id`. \
     This query is already executed server-side to build the `lines` CSV \
     for `upcomingbuses`.",
    Category::GtfsReplaceable,
    format!("{base}/buses/busstops/{STOP_ID}/lines"),
    format!("busstopId={STOP_ID} (CORUÑA y PURIFICACION)"),
  )
  .await;

  let realtime: Vec<Hit> = vec![buses.truncate_body(5), upcomingbuses];

  let replaceable: Vec<Hit> =
    vec![linevariants.truncate_body(3), linevariant_detail, busstops.truncate_body(3), busstop_lines];

  let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
  let total = realtime.len() + replaceable.len();
  let ok = realtime.iter().chain(&replaceable).filter(|r| (200..300).contains(&r.status)).count();
  let err = total - ok;

  let mut md = String::new();

  writeln!(md, "# IM Buses API — Endpoint Report")?;
  writeln!(md)?;
  writeln!(md, "| | |")?;
  writeln!(md, "|---|---|")?;
  writeln!(md, "| **Generated** | {now} |")?;
  writeln!(md, "| **Base URL** | `{base}` |")?;
  writeln!(md, "| **Auth** | OAuth2 Client Credentials · token TTL 300 s |")?;
  writeln!(md, "| **Sample stop** | `{STOP_ID}` — CORUÑA y PURIFICACION |")?;
  writeln!(md, "| **Sample line** | `{line}` (discovered) |")?;
  writeln!(md)?;
  writeln!(md, "**{ok}/{total} endpoints OK** · {err} error(s)")?;
  writeln!(md)?;

  writeln!(md, "## Overview")?;
  writeln!(md)?;
  writeln!(md, "| Endpoint | Category | Status |")?;
  writeln!(md, "|---|---|:---:|")?;
  for r in realtime.iter().chain(&replaceable) {
    let cat = match r.category {
      Category::RealtimeOnly => "🔴 Realtime-only",
      Category::GtfsReplaceable => "🟢 GTFS replaceable",
    };
    let badge = match r.status {
      0 => "❌".into(),
      200..=299 => format!("✅ {}", r.status),
      _ => format!("⚠️ {}", r.status),
    };
    writeln!(md, "| `{}` | {} | {} |", r.endpoint, cat, badge)?;
  }
  writeln!(md)?;

  writeln!(md, "---")?;
  writeln!(md)?;
  writeln!(md, "## Necessary even with GTFS — Real-time data")?;
  writeln!(md)?;
  writeln!(
    md,
    "These endpoints provide live information that **does not exist in the GTFS static feed**. \
     They must always be called to show real-time ETAs and vehicle positions."
  )?;
  writeln!(md)?;

  for r in &realtime {
    render_hit(&mut md, r)?;
  }

  writeln!(md, "---")?;
  writeln!(md)?;
  writeln!(md, "## Replaceable by GTFS static data")?;
  writeln!(md)?;
  writeln!(
    md,
    "> **Disclaimer:** The endpoints below return data that is **entirely covered by the \
     GTFS feed** (`google_transit.zip`). Calling them against the IM API is redundant once \
     the feed is loaded into the local database. They are documented here for completeness \
     and to validate field parity between the IM API and the GTFS tables."
  )?;
  writeln!(md)?;

  for r in &replaceable {
    render_hit(&mut md, r)?;
  }

  let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("out").join("im_api_report.md");
  std::fs::create_dir_all(out.parent().unwrap())?;
  std::fs::write(&out, &md)?;
  eprintln!("\n✅  Report → {}", out.display());

  Ok(())
}
