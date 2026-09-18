// https://api.montevideo.gub.uy/apidocs/publictransport#/gtfs
// https://gtfs.org/
mod model;
use model::LocalGtfs;

#[derive(Debug, PartialEq)]
enum SyncStatus {
  UpToDate,
  Synced,
}

async fn check_and_sync(state: &crate::State) -> crate::Result<SyncStatus> {
  use crate::im_api::GtfsData;
  use gtfs_structures::Gtfs;
  use std::io::Cursor;

  let new_version = state.im_api.gtfs_version().await?;
  let current_version = LocalGtfs::version(&state.db).await?;

  if current_version.as_deref() == Some(new_version.as_str()) {
    tracing::info!(new_version, "[GTFS] up to date");
    return Ok(SyncStatus::UpToDate);
  }

  // download
  tracing::info!(new = new_version, previous = ?current_version, "[GTFS] syncing");
  let zip_bytes = state.im_api.gtfs_download().await?;
  let gtfs = tokio::task::spawn_blocking(move || Gtfs::from_reader(Cursor::new(zip_bytes)))
    .await
    .map_err(|e| crate::Error::ExternalApi(format!("[GTFS] parse task panicked: {e}")))?
    .map_err(|e| crate::Error::ExternalApi(format!("[GTFS] parse failed: {e}")))?;

  // truncate & insert
  LocalGtfs::replace_transaction(&state.db, &gtfs, &new_version).await?;

  {
    // stop bearings, outside the transaction
    match sqlx::query(
      r#"
    UPDATE stops s
    SET bearing_rad = sub.rad
    FROM (SELECT DISTINCT ON (pairs.stop_id)
      pairs.stop_id,
      ST_Azimuth(s1.geom, s2.geom) AS rad
      FROM (SELECT stop_id, LEAD(stop_id) OVER (PARTITION BY trip_id ORDER BY stop_sequence) AS next_stop_id
        FROM stop_times) pairs
          JOIN stops s1 ON s1.stop_id = pairs.stop_id
          JOIN stops s2 ON s2.stop_id = pairs.next_stop_id
        WHERE pairs.next_stop_id IS NOT NULL
          AND ST_Distance(s1.geom, s2.geom) > 0
        ORDER BY pairs.stop_id
    ) sub WHERE s.stop_id = sub.stop_id
    "#,
    )
    .execute(&state.db)
    .await
    {
      Ok(r) => tracing::info!(rows = r.rows_affected(), "[GTFS] stop bearings computed"),
      Err(e) => tracing::warn!(error = %e, "[GTFS] stop bearing computation failed (non-fatal)"),
    }
  }

  Ok(SyncStatus::Synced)
}

pub fn spawn_scheduler(state: crate::State) {
  tokio::spawn(async move {
    run_once(&state).await;

    loop {
      let wait = std::time::Duration::from_secs(state.cache.until_sync());

      tracing::info!("[GTFS] next check in {wait:?}");
      tokio::time::sleep(wait).await;
      run_once(&state).await;
    }
  });
}

async fn run_once(state: &crate::State) {
  match check_and_sync(state).await {
    Ok(status) => {
      tracing::info!("[GTFS] check complete");
      if status == SyncStatus::Synced {
        use crate::cache::Flush;
        state.cache.flush().await;
      }
    }
    Err(e) => tracing::error!("[GTFS] sync failed: {e:?}"),
  }
}
