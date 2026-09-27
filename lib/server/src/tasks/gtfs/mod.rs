// https://api.montevideo.gub.uy/apidocs/publictransport#/gtfs
// https://gtfs.org/
mod model;
mod times;
use model::LocalGtfs;

#[derive(Debug, PartialEq)]
enum SyncStatus {
  UpToDate,
  Synced(String),
}

async fn check_and_sync(state: &crate::State, handle: &crate::core::provider::Provider) -> crate::Result<SyncStatus> {
  use gtfs_structures::Gtfs;
  use std::io::Cursor;

  let provider = handle.id;
  let Some(feed) = handle.feed.as_deref() else {
    tracing::info!(provider, "[gtfs] provider has no feed");
    return Ok(SyncStatus::UpToDate);
  };

  let new_version = feed.version().await?;
  let current_version = LocalGtfs::version(&state.tasks_db, provider).await?;

  if current_version.as_deref() == Some(new_version.as_str()) {
    return Ok(SyncStatus::UpToDate);
  }

  // download
  tracing::info!(provider, new = new_version, previous = ?current_version, "[gtfs] syncing");
  let crate::core::provider::Static { gtfs: zip_bytes, lines_by_stop } = feed.download().await?;
  let gtfs = tokio::task::spawn_blocking(move || Gtfs::from_reader(Cursor::new(zip_bytes)))
    .await
    .map_err(|e| crate::Error::External(format!("GTFS parse task panicked: {e}")))?
    .map_err(|e| crate::Error::External(format!("GTFS parse failed: {e}")))?;

  // this provider only
  LocalGtfs::replace_transaction(
    &state.tasks_db,
    &gtfs,
    &new_version,
    provider,
    handle.authority,
    feed,
    lines_by_stop.as_ref(),
  )
  .await?;

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
        FROM stop_times WHERE region = $1 AND provider = $2) pairs
          JOIN stops s1 ON s1.region = $1 AND s1.provider = $2 AND s1.stop_id = pairs.stop_id
          JOIN stops s2 ON s2.region = $1 AND s2.provider = $2 AND s2.stop_id = pairs.next_stop_id
        WHERE pairs.next_stop_id IS NOT NULL
          AND ST_Distance(s1.geom, s2.geom) > 0
        ORDER BY pairs.stop_id
    ) sub WHERE s.region = $1 AND s.provider = $2 AND s.stop_id = sub.stop_id
    "#,
    )
    .bind(handle.authority.id())
    .bind(provider)
    .execute(&state.tasks_db)
    .await
    {
      Ok(r) => tracing::info!(rows = r.rows_affected(), "[gtfs] stop bearings computed"),
      Err(e) => tracing::warn!(error = %e, "[gtfs] stop bearing computation failed (non-fatal)"),
    }
  }

  Ok(SyncStatus::Synced(new_version))
}

pub fn spawn_scheduler(state: crate::State) {
  tokio::spawn(async move {
    run_once(&state).await;

    loop {
      let wait = std::time::Duration::from_secs(state.cache.until_sync());

      tracing::info!("[gtfs] next check in {wait:?}");
      tokio::time::sleep(wait).await;
      run_once(&state).await;
    }
  });
}

/// Feeds sync independently
async fn run_once(state: &crate::State) {
  let mut synced = false;

  for handle in state.handles() {
    let provider = handle.id.to_string();

    match check_and_sync(state, &handle).await {
      Ok(SyncStatus::Synced(version)) => {
        use crate::cache::Flush;
        state.cache.flush_region(handle.authority).await;
        state.gtfs_versions.write().await.insert(provider.clone(), version);
        synced = true;
        tracing::info!(provider, "[gtfs] check complete");
      }
      Ok(SyncStatus::UpToDate) => tracing::info!(provider, "[gtfs] up to date"),
      Err(e) => tracing::error!(provider, "[gtfs] sync failed: {e:?}"),
    }
  }

  if synced {
    state.web_rebuild().await;

    // all providers, only after a change
    merge_physical_stops(state).await;
  }

  // every time, outlines come from migrations
  assign_terminals(state).await;
  warm_stop_lists(state).await;
}

/// Same shelter across providers (m)
const SAME_STOP_M: f64 = 31.0;

/// Max stop to terminal outline distance (m)
const TERMINAL_REACH_M: f64 = 50.0;

/// Groups stops of one point, the leader keeps name and id
async fn merge_physical_stops(state: &crate::State) {
  use crate::domain::entities::stop::Stop;

  match Stop::rebuild_clusters(&state.tasks_db, &state.priority(), SAME_STOP_M).await {
    Ok(n) => tracing::info!(stops = n, radius_m = SAME_STOP_M, "[stops] shared shelters clustered"),
    Err(e) => tracing::warn!(error = ?e, "[stops] couldn't cluster stops"),
  }

  match Stop::absorb_zonal_schedules(&state.tasks_db, ZONAL_REACH_M).await {
    Ok(n) => tracing::info!(schedules = n, radius_m = ZONAL_REACH_M, "[stops] zonal schedules distributed"),
    Err(e) => tracing::warn!(error = ?e, "[stops] couldn't distribute zonal schedules"),
  }
}

/// Search radius for the real stop of a zone schedule (m)
const ZONAL_REACH_M: f64 = 80.0;

async fn assign_terminals(state: &crate::State) {
  use crate::domain::entities::stop::Stop;

  match Stop::rebuild_terminals(&state.tasks_db, TERMINAL_REACH_M).await {
    Ok(n) => tracing::info!(terminals = n, radius_m = TERMINAL_REACH_M, "[stops] terminal footprints assigned"),
    Err(e) => tracing::warn!(error = ?e, "[stops] couldn't assign terminal footprints"),
  }
}

/// Stop list precompressed ahead of the first visit
async fn warm_stop_lists(state: &crate::State) {
  use crate::domain::entities::stop::Stop;

  // per department, as the map asks
  let areas = match Stop::areas(&state.tasks_db).await {
    Ok(areas) => areas,
    Err(e) => {
      tracing::warn!(error = ?e, "[stops] couldn't list areas");
      return;
    }
  };

  for area in areas {
    if let Err(e) = Stop::warm_list(&state.tasks_db, &state.cache, &area).await {
      tracing::warn!(area = %area, error = ?e, "[stops] couldn't pre-warm list");
    }
  }
}
