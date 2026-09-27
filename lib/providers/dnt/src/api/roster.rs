use crate::sdk::prelude::*;

use serde::Deserialize;

const LAYER: &str = "inf_tte_pasajeros:v_paradas_suburbanos";
const URL: &str = concat!(
  "https://geoservicios.mtop.gub.uy/geoserver/wfs",
  "?service=WFS&version=2.0.0&request=GetFeature",
  "&typeNames=inf_tte_pasajeros:v_paradas_suburbanos",
  "&outputFormat=application/json"
);

#[derive(Deserialize)]
struct Collection {
  #[serde(default)]
  features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
  properties: Properties,
}

#[derive(Deserialize)]
struct Properties {
  #[serde(rename = "codigo")]
  code: Option<String>,
  #[serde(rename = "lineas")]
  lines: Option<String>,
}

pub async fn lines_by_stop(client: &reqwest::Client) -> Result<LinesByStop> {
  let res = http::fetch(client.get(URL), "[dnt/roster]").await?;
  let bytes = http::bytes(res, "[dnt/roster]").await?;
  let by_stop = parse(&bytes)?;

  if by_stop.is_empty() {
    tracing::warn!(layer = LAYER, "[dnt/roster] empty collection");
    return Err(Error::External(format!("{LAYER} returned no stops")));
  }

  tracing::info!(stops = by_stop.len(), "[dnt/roster] OK");
  Ok(by_stop)
}

pub fn parse(bytes: &[u8]) -> Result<LinesByStop> {
  let collection: Collection =
    serde_json::from_slice(bytes).map_err(|e| Error::External(format!("unreadable roster: {e}")))?;

  Ok(collection.features.into_iter().filter_map(Feature::into_stop_lines).collect())
}

impl Feature {
  fn into_stop_lines(self) -> Option<(String, Vec<String>)> {
    let Properties { code, lines } = self.properties;

    let code = code?.trim().to_string();
    if code.is_empty() {
      return None;
    }

    let lines: Vec<String> = lines?.split('-').map(format::line_name).filter(|l| !l.is_empty()).collect();

    (!lines.is_empty()).then_some((code, lines))
  }
}
