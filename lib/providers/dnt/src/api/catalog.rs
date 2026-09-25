// https://catalogodatos.gub.uy CKAN
use crate::core::provider::prelude::*;

use serde::Deserialize;

pub const SCHEDULES: &str =
  "ministerio-de-transporte-y-obras-publicas-horarios-de-omnibus-en-lineas-interdepartamentales";
pub const SHAPES: &str = "ministerio-de-transporte-y-obras-publicas-recorridos-omnibus-suburbanos";

const CKAN: &str = "https://catalogodatos.gub.uy/api/3/action/package_show?id=";

#[derive(Deserialize)]
struct Package {
  result: PackageResult,
}

#[derive(Deserialize)]
struct PackageResult {
  resources: Vec<Resource>,
}

#[derive(Deserialize, Clone)]
pub struct Resource {
  pub url: String,
  #[serde(default)]
  pub last_modified: Option<String>,
}

impl Resource {
  pub fn file_name(&self) -> &str {
    self.url.rsplit('/').next().unwrap_or_default()
  }
}

pub async fn resource(client: &reqwest::Client, dataset: &str, prefix: &str) -> Result<Resource> {
  let res = http::fetch(client.get(format!("{CKAN}{dataset}")), "[dnt/catalog]").await?;
  let package: Package = http::json(res, "[dnt/catalog]").await?;

  package.result.resources.into_iter().find(|r| r.file_name().starts_with(prefix)).ok_or_else(|| {
    tracing::warn!(dataset, prefix, "[dnt/catalog] resource not published");
    Error::External(format!("{dataset} publishes no {prefix}*"))
  })
}

pub async fn download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
  let res = http::fetch(client.get(url), "[dnt/download]").await?;
  http::bytes(res, "[dnt/download]").await
}
