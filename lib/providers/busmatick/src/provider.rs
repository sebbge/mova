use crate::BusmatickConfig;
use crate::api::colonia::{self, Shapes};
use crate::api::{self, Marker, codesa};
use crate::core::provider::prelude::*;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct Busmatick {
  http: reqwest::Client,
  base: String,
  company: &'static str,
  tag: String,
}
impl Busmatick {
  pub fn new(id: &str, company: &'static str, config: &BusmatickConfig) -> Result<Self> {
    Ok(Self::with(http::client(Duration::from_secs(15))?, id, company, config))
  }

  pub fn legacy_tls(id: &str, company: &'static str, config: &BusmatickConfig) -> Result<Self> {
    Ok(Self::with(http::legacy_tls_client(Duration::from_secs(15))?, id, company, config))
  }

  fn with(http: reqwest::Client, id: &str, company: &'static str, config: &BusmatickConfig) -> Self {
    Self { http, base: config.base.clone(), company, tag: format!("[{id}/avl]") }
  }

  async fn markers(&self) -> Result<Vec<Marker>> {
    api::markers(&self.http, &self.base, self.company, &self.tag).await
  }
}

#[async_trait::async_trait]
impl Fleet for Busmatick {
  async fn fleet(&self) -> Result<Vec<BusEntry>> {
    Ok(self.markers().await?.into_iter().map(BusEntry::from).collect())
  }
}

pub struct Codesa {
  http: reqwest::Client,
  base: String,
}
impl Codesa {
  pub fn new(config: &BusmatickConfig) -> Result<Self> {
    Ok(Self { http: http::client(Duration::from_secs(60))?, base: config.base.clone() })
  }
}

#[async_trait::async_trait]
impl Feed for Codesa {
  async fn version(&self) -> Result<String> {
    Ok(codesa::version(&codesa::sheet(&self.http).await?))
  }

  async fn download(&self) -> Result<Static> {
    let trips = codesa::parse_sheet(&codesa::sheet(&self.http).await?)?;
    let variants = codesa::variants(&self.http, &self.base, &trips).await;

    let gtfs = tokio::task::spawn_blocking(move || codesa::synthesize(&trips, &variants))
      .await
      .map_err(|e| Error::External(format!("feed synthesis panicked: {e}")))??;
    Ok(Static { gtfs, lines_by_stop: None })
  }
}

pub struct Solantigua {
  http: reqwest::Client,
  avl: Busmatick,
  shapes: RwLock<Option<Arc<Shapes>>>,
}
impl Solantigua {
  pub fn new(config: &BusmatickConfig) -> Result<Self> {
    let avl = Busmatick::new("solantigua", "SOL ANTIGUA", config)?;
    Ok(Self { http: http::client(Duration::from_secs(60))?, avl, shapes: RwLock::new(None) })
  }

  async fn shapes(&self) -> Result<Arc<Shapes>> {
    if let Some(shapes) = self.shapes.read().await.as_ref() {
      return Ok(shapes.clone());
    }
    let (_, shapes) = index(colonia::download(&self.http).await?).await?;
    let shapes = Arc::new(shapes?);
    *self.shapes.write().await = Some(shapes.clone());
    Ok(shapes)
  }
}

async fn index(zip: Vec<u8>) -> Result<(Vec<u8>, Result<Shapes>)> {
  tokio::task::spawn_blocking(move || {
    let shapes = Shapes::from_zip(&zip);
    (zip, shapes)
  })
  .await
  .map_err(|e| Error::External(format!("shape index panicked: {e}")))
}

#[async_trait::async_trait]
impl Feed for Solantigua {
  async fn version(&self) -> Result<String> {
    colonia::version(&self.http).await
  }

  async fn download(&self) -> Result<Static> {
    let (gtfs, shapes) = index(colonia::download(&self.http).await?).await?;
    match shapes {
      Ok(shapes) => *self.shapes.write().await = Some(Arc::new(shapes)),
      Err(e) => tracing::warn!(error = ?e, "[solantigua/gtfs] no shape index, buses go unbound"),
    }
    Ok(Static { gtfs, lines_by_stop: None })
  }
}

#[async_trait::async_trait]
impl Fleet for Solantigua {
  async fn fleet(&self) -> Result<Vec<BusEntry>> {
    let (markers, shapes) = tokio::join!(self.avl.markers(), self.shapes());
    let shapes = shapes.inspect_err(|e| tracing::warn!(error = ?e, "[solantigua/gtfs] shape index unavailable")).ok();
    let buses = markers?.into_iter().map(|m| m.into_bus(|line, subline| shapes.as_ref()?.shape(line, subline)));
    Ok(buses.collect())
  }
}
