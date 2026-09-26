mod kmz;
mod points;
mod sheet;
mod synthesis;

use crate::core::provider::prelude::*;

pub use kmz::{Stop, kml, shape, stops};
pub use points::table;
pub use sheet::{Trip, parse as parse_sheet};
pub use synthesis::{Variant, Variants, anchor, gtfs as synthesize};

use futures_util::StreamExt;
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

const CONCURRENCY: usize = 8;

//# GET <GSHEETS>/gviz/tq?sheet=BASE_HORARIOS
pub async fn sheet(client: &reqwest::Client) -> Result<Vec<u8>> {
  let res = http::fetch(client.get(sheet::URL), "[codesa/sheet]").await?;
  http::bytes(res, "[codesa/sheet]").await
}

pub fn version(sheet: &[u8]) -> String {
  let mut hasher = DefaultHasher::new();
  sheet.hash(&mut hasher);
  format!("{:x}+{:x}", hasher.finish(), points::fingerprint())
}

pub async fn variants(client: &reqwest::Client, base: &str, trips: &[Trip]) -> Variants {
  let combos: BTreeSet<(String, String)> = trips.iter().map(|v| (v.line.clone(), v.subline.clone())).collect();
  let total = combos.len();

  let variants: Variants = futures_util::stream::iter(combos)
    .map(|(line, subline)| async move {
      let variant = variant(client, base, &line, &subline).await;
      variant.map(|v| ((line, subline), v))
    })
    .buffer_unordered(CONCURRENCY)
    .filter_map(|v| async move { v })
    .collect()
    .await;

  let without_shape = variants.values().filter(|v| v.shape.is_none()).count();
  if variants.len() < total || without_shape > 0 {
    tracing::warn!(
      without_stops = total - variants.len(),
      without_shape = without_shape,
      "[codesa/kmz] incomplete variants"
    );
  }
  variants
}

async fn variant(client: &reqwest::Client, base: &str, line: &str, subline: &str) -> Option<Variant> {
  let (stops, shape) =
    tokio::join!(kmz(client, base, "Paradas", line, subline), kmz(client, base, "Recorridos", line, subline));
  let stops = kmz::stops(&stops?);
  if stops.len() < 2 {
    return None;
  }
  Some(Variant { stops, shape: shape.as_deref().and_then(kmz::shape) })
}

//# GET <CODESA>/{Paradas,Recorridos}/{line}_{subline}.kmz
async fn kmz(client: &reqwest::Client, base: &str, folder: &str, line: &str, subline: &str) -> Option<String> {
  let res = http::send(client.get(format!("{base}/{folder}/{line}_{subline}.kmz")), "[codesa/kmz]").await.ok()?;
  if res.status() == reqwest::StatusCode::NOT_FOUND {
    return None;
  }
  let res = http::ok(res, "[codesa/kmz]").await.ok()?;
  let bytes = http::bytes(res, "[codesa/kmz]").await.ok()?;
  kmz::kml(&bytes).inspect_err(|e| tracing::warn!(line, subline, folder, error = ?e, "[codesa/kmz] unreadable")).ok()
}
