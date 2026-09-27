use super::{Bus, Headsigns, Line};
use crate::scope::Scope;

//- cache
use crate::cache::CacheRw;
impl Line {
  const TTL: u64 = 10;

  /// Per provider and region
  fn buses_key(region: &str, provider: &str, line_id: &str) -> String {
    crate::cache::key(region, &format!("rt:{provider}:buses:{line_id}"))
  }

  pub(crate) async fn cache_get(cache: &crate::Cache, region: &str, provider: &str, line_id: &str) -> Option<Self> {
    let buses: Vec<Bus> = cache.get(&Self::buses_key(region, provider, line_id)).await?;
    let destination = buses.first().map(|b| b.seen.destination.clone()).unwrap_or_default();
    let company = buses.first().map(|b| b.seen.company.clone()).unwrap_or_default();
    Some(Self { line_id: line_id.to_string(), buses, company, destination })
  }

  pub(crate) async fn cache_set(cache: &crate::Cache, region: &str, provider: &str, line_id: &str, buses: &[Bus]) {
    cache.set(&Self::buses_key(region, provider, line_id), &buses, Self::TTL).await;
  }

  //
  fn headsign_key(region: &str, provider: &str, route_short_name: &str) -> String {
    crate::cache::key(region, &format!("headsigns:{provider}:{route_short_name}"))
  }

  pub(crate) async fn get_headsigns_cache(
    cache: &crate::Cache,
    region: &str,
    provider: &str,
    route_short_name: &str,
  ) -> Option<Headsigns> {
    cache.get(&Self::headsign_key(region, provider, route_short_name)).await
  }

  pub(crate) async fn set_headsigns_cache(
    cache: &crate::Cache,
    region: &str,
    provider: &str,
    route_short_name: &str,
    hs: &Headsigns,
  ) {
    cache.set(&Self::headsign_key(region, provider, route_short_name), hs, cache.until_sync()).await;
  }
}

//- db
#[derive(sqlx::FromRow)]
pub struct BearingRow {
  pub bus_id: String,
  pub bearing_deg: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub struct HeadsignRow {
  pub direction_id: i16,
  pub trip_headsign: Option<String>,
}

impl Line {
  pub(super) async fn query_bearings(
    db: &crate::Db,
    scope: &Scope,
    line: &str,
    ids: &[String],
    lats: &[f64],
    lngs: &[f64],
    dests: &[Option<String>],
  ) -> Vec<BearingRow> {
    let q = r#"
      WITH line_shapes AS (
        SELECT DISTINCT ON (t.shape_id)
            t.trip_headsign AS headsign,
            sh.geom         AS shape
        FROM routes r
        JOIN trips  t  ON t.route_id  = r.route_id
        JOIN shapes sh ON sh.shape_id = t.shape_id
        WHERE r.region = $6 AND t.region = $6 AND sh.region = $6
          AND r.provider = $7 AND t.provider = $7 AND sh.provider = $7
          AND r.route_short_name = $1
          AND t.direction_id IS NOT NULL
        ORDER BY t.shape_id
    ),
    buses AS (
        SELECT * FROM unnest($2::text[], $3::float8[], $4::float8[], $5::text[])
          AS t(bus_id, bus_lat, bus_lng, bus_dest)
    )
    SELECT DISTINCT ON (b.bus_id)
        b.bus_id,
        degrees(ST_Azimuth(
            ST_LineInterpolatePoint(
                ls.shape::geometry,
                GREATEST(0, ST_LineLocatePoint(ls.shape::geometry,
                    ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) - 0.001)
            ),
            ST_LineInterpolatePoint(
                ls.shape::geometry,
                LEAST(1, ST_LineLocatePoint(ls.shape::geometry,
                    ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) + 0.001)
            )
        )) AS bearing_deg
    FROM buses b
    JOIN line_shapes ls ON (
        b.bus_dest IS NULL
        OR ls.headsign IS NULL
        OR UPPER(TRIM(ls.headsign)) = UPPER(TRIM(b.bus_dest))
    )
    WHERE ST_Distance(
        ls.shape::geography,
        ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography
    ) < 500
    ORDER BY b.bus_id,
        ST_Distance(
            ls.shape::geography,
            ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography
        )"#;
    sqlx::query_as(q)
      .bind(line)
      .bind(ids)
      .bind(lats)
      .bind(lngs)
      .bind(dests)
      .bind(scope.region.id())
      .bind(scope.provider)
      .fetch_all(db)
      .await
      .unwrap_or_default()
  }

  pub(super) async fn query_headsigns(db: &crate::Db, scope: &Scope, route_short_name: &str) -> Vec<HeadsignRow> {
    let q = r#"
      SELECT DISTINCT t.direction_id, t.trip_headsign
      FROM trips t
      JOIN routes r ON r.route_id = t.route_id
      WHERE r.region = $2 AND t.region = $2 AND r.provider = $3 AND t.provider = $3
        AND r.route_short_name = $1
        AND t.direction_id IS NOT NULL
        AND t.trip_headsign IS NOT NULL
      ORDER BY t.direction_id, t.trip_headsign
      "#;
    sqlx::query_as(q)
      .bind(route_short_name)
      .bind(scope.region.id())
      .bind(scope.provider)
      .fetch_all(db)
      .await
      .unwrap_or_default()
  }
}
