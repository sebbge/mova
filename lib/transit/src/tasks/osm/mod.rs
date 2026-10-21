// https://download.geofabrik.de/south-america/uruguay.html
mod model;
mod pbf;
use model::Row;

use std::time::Duration;

const SOURCE: &str = "https://download.geofabrik.de/south-america/uruguay-latest.osm.pbf";
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);
const CHECK: Duration = Duration::from_secs(24 * 3600);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const CHUNK: usize = 20_000;

pub fn spawn_scheduler(state: crate::Transit) {
  tokio::spawn(async move {
    loop {
      match refresh(&state).await {
        Ok(Some(places)) => tracing::info!(places, "[places] synced"),
        Ok(None) => tracing::info!("[places] up to date"),
        Err(e) => tracing::error!("[places] sync failed: {e:?}"),
      }
      tokio::time::sleep(CHECK).await;
    }
  });
}

async fn refresh(state: &crate::Transit) -> crate::Result<Option<usize>> {
  let age: Option<f64> =
    sqlx::query_scalar("SELECT EXTRACT(EPOCH FROM now() - loaded_at)::float8 FROM places_metadata LIMIT 1")
      .fetch_optional(&state.tasks_db)
      .await?;
  if age.is_some_and(|secs| secs < MAX_AGE.as_secs_f64()) {
    return Ok(None);
  }

  let external = |e: reqwest::Error| crate::Error::External(format!("OSM download failed: {e}"));
  let client = reqwest::Client::builder().timeout(DOWNLOAD_TIMEOUT).build().map_err(external)?;
  let res = client.get(SOURCE).send().await.and_then(|r| r.error_for_status()).map_err(external)?;
  let source = res.url().path().rsplit('/').next().unwrap_or_default().to_string();
  let bytes = res.bytes().await.map_err(external)?;
  tracing::info!(source, bytes = bytes.len(), "[places] downloaded");

  let rows = tokio::task::spawn_blocking(move || model::extract(&bytes))
    .await
    .map_err(|e| crate::Error::External(format!("OSM parse task panicked: {e}")))?
    .map_err(|e| crate::Error::External(format!("OSM parse failed: {e}")))?;

  replace(&state.tasks_db, &rows, &source).await?;
  Ok(Some(rows.len()))
}

/// All at once, search keeps the old places meanwhile
async fn replace(db: &crate::Db, rows: &[Row], source: &str) -> crate::Result<()> {
  let mut tx = db.begin().await?;
  sqlx::query("DELETE FROM places").execute(tx.as_mut()).await?;

  let q = r#"
    INSERT INTO places (id, kind, name, category, context, key, crossing, rank, geom, area)
    SELECT t.id, t.kind, t.name, t.category, COALESCE(t.context, a.name), t.key, t.crossing, t.rank, p.g::geography, a.code
    FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[], $8::int2[],
                $9::float8[], $10::float8[])
      AS t(id, kind, name, category, context, key, crossing, rank, lng, lat)
    CROSS JOIN LATERAL (SELECT ST_SetSRID(ST_MakePoint(t.lng, t.lat), 4326) AS g) p
    -- the Geofabrik extract crosses the border
    JOIN LATERAL (
      SELECT ap.code, ar.name FROM area_parts ap JOIN areas ar ON ar.code = ap.code
      WHERE ST_Intersects(ap.geom, p.g) LIMIT 1
    ) a ON true
    "#;
  for chunk in rows.chunks(CHUNK) {
    let c = Row::columns(chunk);
    sqlx::query(q)
      .bind(&c.ids)
      .bind(&c.kinds)
      .bind(&c.names)
      .bind(&c.categories)
      .bind(&c.contexts)
      .bind(&c.keys)
      .bind(&c.crossings)
      .bind(&c.ranks)
      .bind(&c.lngs)
      .bind(&c.lats)
      .execute(tx.as_mut())
      .await?;
  }

  sqlx::query("DELETE FROM places_metadata").execute(tx.as_mut()).await?;
  sqlx::query("INSERT INTO places_metadata (source, loaded_at) VALUES ($1, now())")
    .bind(source)
    .execute(tx.as_mut())
    .await?;
  tx.commit().await?;
  sqlx::query("ANALYZE places").execute(db).await?;
  Ok(())
}
