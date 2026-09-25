mod catalog;
mod feed;
mod kml;
mod online;
mod roster;

use crate::core::provider::prelude::*;

pub use feed::rebuild as rebuild_feed;
pub use kml::{Shape, parse as parse_kml};
pub use online::OnlineBus;
pub use roster::{lines_by_stop as fetch_roster, parse as parse_roster};

use std::time::Duration;

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

  pub async fn buses(&self) -> Result<Vec<OnlineBus>> {
    online::buses(&self.http).await
  }

  pub async fn lines_by_stop(&self) -> Result<LinesByStop> {
    roster::lines_by_stop(&self.http).await
  }
}
