use super::{Bus, Headsigns, Line, Positions};

//- cache
impl Line {
  const TTL: u64 = 10;

  /// Per provider and region
  fn buses_key(region: &str, provider: &str, line_id: &str) -> String {
    crate::cache::Cache::key(region, &format!("rt:{provider}:buses:{line_id}"))
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
    crate::cache::Cache::key(region, &format!("headsigns:{provider}:{route_short_name}"))
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
use crate::scope::Scope;
#[derive(sqlx::FromRow)]
pub struct BearingRow {
  pub bus_id: String,
  pub bearing_deg: Option<f64>,
}

#[derive(sqlx::FromRow)]
pub struct NextStopRow {
  pub bus_id: String,
  pub stop_id: String,
  pub stop_name: String,
}

#[derive(sqlx::FromRow)]
pub struct HeadsignRow {
  pub direction_id: i16,
  pub trip_headsign: Option<String>,
}

impl Line {
  /// Rewritten only when older than an hour
  pub(crate) async fn remember_companies<'a>(db: &crate::Db, scope: &Scope, buses: impl Iterator<Item = &'a Bus>) {
    let mut pairs: Vec<(&str, &str)> = buses
      .filter(|b| !b.seen.line.is_empty() && !b.seen.company.is_empty())
      .map(|b| (b.seen.line.as_str(), b.seen.company.as_str()))
      .collect();
    pairs.sort_unstable();
    pairs.dedup();
    if pairs.is_empty() {
      return;
    }
    let (lines, companies): (Vec<&str>, Vec<&str>) = pairs.into_iter().unzip();

    let q = r#"
      INSERT INTO line_companies (region, provider, line, company)
      SELECT $1, $2, t.line, t.company FROM UNNEST($3::text[], $4::text[]) AS t(line, company)
      ON CONFLICT (region, provider, line, company) DO UPDATE SET seen_at = now()
      WHERE line_companies.seen_at < now() - INTERVAL '1 hour'
    "#;
    let result =
      sqlx::query(q).bind(scope.region.id()).bind(scope.provider).bind(&lines).bind(&companies).execute(db).await;
    if let Err(e) = result {
      tracing::warn!(provider = scope.provider, error = ?e, "[fleet] line companies not saved");
    }
  }

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

  pub(super) async fn query_next_stops(db: &crate::Db, scope: &Scope, line: &str, p: &Positions) -> Vec<NextStopRow> {
    let q = r#"
      WITH line_shapes AS (
        SELECT DISTINCT ON (t.shape_id)
            t.trip_id, t.trip_headsign AS headsign, sh.geom AS shape
        FROM routes r
        JOIN trips  t  ON t.region = r.region AND t.provider = r.provider AND t.route_id = r.route_id
        JOIN shapes sh ON sh.region = t.region AND sh.provider = t.provider AND sh.shape_id = t.shape_id
        WHERE r.region = $6 AND r.provider = $7 AND r.route_short_name = $1
          AND t.direction_id IS NOT NULL
        ORDER BY t.shape_id, t.trip_id
      ),
      buses AS (
        SELECT bus_id, ST_SetSRID(ST_MakePoint(bus_lng, bus_lat), 4326) AS pt, bus_dest
        FROM unnest($2::text[], $3::float8[], $4::float8[], $5::text[]) AS t(bus_id, bus_lat, bus_lng, bus_dest)
      ),
      placed AS (
        SELECT DISTINCT ON (b.bus_id)
            b.bus_id, ls.trip_id, ls.shape, ST_LineLocatePoint(ls.shape::geometry, b.pt) AS frac
        FROM buses b
        JOIN line_shapes ls ON (
            b.bus_dest IS NULL
            OR ls.headsign IS NULL
            OR UPPER(TRIM(ls.headsign)) = UPPER(TRIM(b.bus_dest))
        )
        WHERE ST_DWithin(ls.shape::geography, b.pt::geography, 250)
        ORDER BY b.bus_id, ST_Distance(ls.shape::geography, b.pt::geography)
      )
      SELECT DISTINCT ON (p.bus_id) p.bus_id, s.stop_id, s.stop_name
      FROM placed p
      JOIN stop_times st ON st.region = $6 AND st.provider = $7 AND st.trip_id = p.trip_id
      JOIN stops s ON s.region = st.region AND s.provider = st.provider AND s.stop_id = st.stop_id
      WHERE ST_LineLocatePoint(p.shape::geometry, s.geom::geometry) > p.frac
      ORDER BY p.bus_id, st.stop_sequence"#;
    sqlx::query_as(q)
      .bind(line)
      .bind(&p.ids)
      .bind(&p.lats)
      .bind(&p.lngs)
      .bind(&p.dests)
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
