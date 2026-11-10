mod catalog;
mod feed;
mod index;
mod kml;
mod long_distance;
mod online;
mod places;
mod roster;
mod seguitubus;
mod synthesis;

use crate::sdk::prelude::*;

pub use index::Index;
pub use long_distance::{Trip, corridors};
pub use online::OnlineBus;
pub use places::all as places;
pub use seguitubus::Service;
pub use synthesis::gtfs as synthesize;

use std::time::Duration;

const LONG_DISTANCE: &str = "horarios_largadistancia";

#[derive(Clone)]
pub struct DntApi {
  http: reqwest::Client,
}

impl DntApi {
  pub fn new() -> Result<Self> {
    Ok(Self { http: http::client(Duration::from_secs(60))? })
  }

  pub async fn gtfs_version(&self) -> Result<String> {
    let resource = catalog::resource(&self.http, catalog::SCHEDULES, "gtfs_").await?;
    Ok(resource.last_modified.clone().unwrap_or_else(|| resource.file_name().to_string()))
  }

  pub async fn gtfs_download(&self) -> Result<Vec<u8>> {
    let (gtfs, shapes) = tokio::join!(
      catalog::resource(&self.http, catalog::SCHEDULES, "gtfs_"),
      catalog::resource(&self.http, catalog::SHAPES, "recorridos_metropolitanos")
    );

    let (gtfs, shapes) = (gtfs?, shapes?);
    let (zip_bytes, kml_bytes) =
      tokio::join!(catalog::download(&self.http, &gtfs.url), catalog::download(&self.http, &shapes.url));
    let (zip_bytes, kml_bytes) = (zip_bytes?, kml_bytes?);

    tokio::task::spawn_blocking(move || {
      let shapes = kml::parse(&kml_bytes);
      tracing::info!(shapes = shapes.len(), "[dnt/feed] KML geometry parsed");
      feed::rebuild(zip_bytes, shapes)
    })
    .await
    .map_err(|e| Error::External(format!("feed rebuild panicked: {e}")))?
  }

  pub async fn long_distance_version(&self) -> Result<String> {
    let resource = catalog::resource(&self.http, catalog::SCHEDULES, LONG_DISTANCE).await?;
    let published = resource.last_modified.clone().unwrap_or_else(|| resource.file_name().to_string());
    Ok(format!("{published}+{:x}", places::fingerprint()))
  }

  pub async fn long_distance(&self) -> Result<Vec<Trip>> {
    let resource = catalog::resource(&self.http, catalog::SCHEDULES, LONG_DISTANCE).await?;
    let bytes = catalog::download(&self.http, &resource.url).await?;
    tokio::task::spawn_blocking(move || long_distance::parse(&bytes))
      .await
      .map_err(|e| Error::External(format!("long-distance parse panicked: {e}")))?
  }

  pub async fn services(&self) -> Result<Vec<Service>> {
    seguitubus::services(&self.http).await
  }

  pub async fn buses(&self) -> Result<Vec<OnlineBus>> {
    online::buses(&self.http).await
  }

  pub async fn lines_by_stop(&self) -> Result<LinesByStop> {
    roster::lines_by_stop(&self.http).await
  }
}
