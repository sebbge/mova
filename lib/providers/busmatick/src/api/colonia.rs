// gtfs.zip of the latest manufarfaro/colonia-gtfs release, data CC BY 4.0
use crate::api::csv::fields;
use crate::core::provider::prelude::*;

use reqwest::header::{ETAG, LAST_MODIFIED};
use std::collections::HashMap;
use std::io::{Cursor, Read};

pub const URL: &str = "https://github.com/manufarfaro/colonia-gtfs/releases/latest/download/gtfs.zip";
const TAG: &str = "[solantigua/gtfs]";

//# HEAD <GITHUB>/colonia-gtfs/releases/latest/download/gtfs.zip
pub async fn version(client: &reqwest::Client) -> Result<String> {
  let res = http::fetch(client.head(URL), TAG).await?;
  let headers = res.headers();
  let version = headers.get(LAST_MODIFIED).or_else(|| headers.get(ETAG)).and_then(|v| v.to_str().ok());
  version.map(str::to_string).ok_or_else(|| Error::External("GTFS release without Last-Modified or ETag".into()))
}

//# GET <GITHUB>/colonia-gtfs/releases/latest/download/gtfs.zip
pub async fn download(client: &reqwest::Client) -> Result<Vec<u8>> {
  let res = http::fetch(client.get(URL), TAG).await?;
  http::bytes(res, TAG).await
}

/// Shape per line and direction, to bind AVL buses
pub struct Shapes(HashMap<(String, u8), String>);

impl Shapes {
  pub fn from_zip(zip: &[u8]) -> Result<Self> {
    let mut archive =
      zip::ZipArchive::new(Cursor::new(zip)).map_err(|e| Error::External(format!("unreadable zip: {e}")))?;
    let mut trips = String::new();
    archive
      .by_name("trips.txt")
      .map_err(|e| Error::External(format!("trips.txt: {e}")))?
      .read_to_string(&mut trips)
      .map_err(|e| Error::External(format!("trips.txt: {e}")))?;

    let mut lines = trips.lines();
    let header = fields(lines.next().unwrap_or_default());
    let col = |name: &str| header.iter().position(|h| h.trim() == name);
    let (Some(route), Some(direction), Some(shape)) = (col("route_id"), col("direction_id"), col("shape_id")) else {
      return Err(Error::External("trips.txt is missing columns".into()));
    };

    let mut out = HashMap::new();
    for line in lines {
      let cols = fields(line);
      let get = |i: usize| cols.get(i).map(|s| s.trim()).unwrap_or_default();
      let (Ok(direction), shape_id) = (get(direction).parse::<u8>(), get(shape)) else {
        continue;
      };
      if !shape_id.is_empty() {
        out.entry((get(route).to_string(), direction)).or_insert_with(|| shape_id.to_string());
      }
    }
    if out.is_empty() {
      return Err(Error::External("trips.txt declares no shapes".into()));
    }
    Ok(Self(out))
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// `direction_id` from the AVL `tra`: 1 -> 1, 2 and 4 -> 0
  pub fn shape(&self, line: &str, subline: &str) -> Option<String> {
    let direction = match subline {
      "1" => 1,
      "2" | "4" => 0,
      _ => return None,
    };
    self.0.get(&(line.to_string(), direction)).cloned()
  }
}
