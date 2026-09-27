// <base>/avl.xml, same on every Busmatick Server instance
use crate::core::provider::prelude::*;

use chrono::{DateTime, Duration, FixedOffset, NaiveDateTime, Utc};

const FORMAT: &str = "%d/%m/%Y %H:%M:%S";
const UTC_OFFSET_S: i32 = -3 * 3600;
const STALE: Duration = Duration::minutes(30);
const AHEAD: Duration = Duration::minutes(5);

pub struct Marker {
  company: &'static str,
  bus: String,
  line: String,
  subline: String,
  headsign: Option<String>,
  location: Point,
  speed: Option<f64>,
  delay: Option<i64>,
  accessible: bool,
  electric: bool,
  timestamp: Option<DateTime<Utc>>,
  current_stop: Option<StopRef>,
  next_stop: Option<StopRef>,
}

//# GET <base>/avl.xml
pub async fn markers(client: &reqwest::Client, base: &str, company: &'static str, tag: &str) -> Result<Vec<Marker>> {
  let res = http::fetch(client.get(format!("{base}/avl.xml")), tag).await?;
  let bytes = http::bytes(res, tag).await?;
  parse(&bytes, company, Utc::now()).inspect_err(|e| tracing::warn!(error = ?e, "{tag} unreadable feed"))
}

pub fn parse(bytes: &[u8], company: &'static str, now: DateTime<Utc>) -> Result<Vec<Marker>> {
  let text: String = bytes.iter().map(|&b| b as char).collect();
  let Some(start) = text.find("<list") else {
    return Err(Error::External("avl.xml has no <list> root".into()));
  };

  Ok(text[start..].split("<marker>").skip(1).filter_map(|block| Marker::parse(block, company, now)).collect())
}

impl Marker {
  fn parse(block: &str, company: &'static str, now: DateTime<Utc>) -> Option<Self> {
    let block = block.split("</marker>").next()?;
    let field = |name: &str| field(block, name);

    let (lat, lng): (f64, f64) = (field("lat")?.parse().ok()?, field("lon")?.parse().ok()?);
    let (bus, line) = (field("bus").filter(|b| !b.is_empty()).or(field("id"))?, field("lin")?);
    if lat == 0.0 || lng == 0.0 || bus.is_empty() || line.is_empty() || line == "0" {
      return None;
    }

    let timestamp = match (field("fec"), field("hor")) {
      (Some(date), Some(time)) => Some(timestamp(date, time)?),
      _ => None,
    };
    if timestamp.is_some_and(|t| t < now - STALE || t > now + AHEAD) {
      return None;
    }

    Some(Marker {
      company,
      bus: bus.to_string(),
      line: line.to_string(),
      subline: field("tra").unwrap_or_default().to_string(),
      headsign: field("lnm").filter(|s| !s.is_empty()).map(str::to_string),
      location: Point { lat, lng },
      speed: field("vel").and_then(|v| v.parse().ok()),
      delay: field("reg").and_then(|v| v.parse().ok()),
      accessible: field("bac") == Some("1"),
      electric: field("ico").is_some_and(|i| i.ends_with('e')),
      timestamp,
      current_stop: stop(field("p1c"), field("p1n")),
      next_stop: stop(field("p2c"), field("p2n")),
    })
  }
}

impl From<Marker> for BusEntry {
  fn from(m: Marker) -> Self {
    m.into_bus(|line, subline| Some(format!("{line}_{subline}")))
  }
}

impl Marker {
  /// `shape_id` resolved by the static source
  pub fn into_bus(self, shape: impl FnOnce(&str, &str) -> Option<String>) -> BusEntry {
    let m = self;
    BusEntry {
      id: BusEntry::identity(m.company, &m.bus),
      shape_id: shape(&m.line, &m.subline),
      unit: m.bus,
      line: format::line_name(&m.line),
      company: m.company.to_string(),
      location: m.location,
      speed: m.speed,
      origin: None,
      destination: m.headsign.as_deref().map(crate::names::destination),
      timestamp: m.timestamp,
      delay_min: m.delay,
      stop: m.current_stop,
      next_stop: m.next_stop,
      access: m.accessible.then(|| "ACCESIBLE".to_string()),
      emissions: m.electric.then(|| "ELÉCTRICO".to_string()),
      thermal_comfort: None,
    }
  }
}

fn field<'a>(block: &'a str, name: &str) -> Option<&'a str> {
  let open = format!("<{name}>");
  let start = block.find(&open)? + open.len();
  let rest = &block[start..];
  Some(rest[..rest.find('<')?].trim())
}

fn stop(code: Option<&str>, name: Option<&str>) -> Option<StopRef> {
  let code = code.filter(|c| !c.is_empty() && *c != "0")?;
  Some(StopRef { id: code.to_string(), name: name.unwrap_or_default().to_string() })
}

fn timestamp(date: &str, time: &str) -> Option<DateTime<Utc>> {
  let local = NaiveDateTime::parse_from_str(&format!("{date} {time}"), FORMAT).ok()?;
  let offset = FixedOffset::east_opt(UTC_OFFSET_S)?;
  Some(local.and_local_timezone(offset).single()?.with_timezone(&Utc))
}
