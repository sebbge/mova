use super::{DirectionInfo, Route, RouteShape, RouteStop};

//- sql
impl Route {
  pub async fn search(db: &crate::Db, q: &str) -> crate::Result<Vec<Self>> {
    let pattern = format!("%{}%", q);

    let routes = sqlx::query_as::<_, Route>(
      r#"
    SELECT route_id, route_short_name, route_long_name, agency_id
    FROM routes
    WHERE route_short_name ILIKE $1
        OR route_long_name  ILIKE $1
    ORDER BY
      CASE WHEN UPPER(route_short_name) = UPPER($2) THEN 0 ELSE 1 END,
      route_short_name
    LIMIT 20
    "#,
    )
    .bind(&pattern)
    .bind(q)
    .fetch_all(db)
    .await?;

    Ok(routes)
  }
}

impl RouteShape {
  pub async fn get(db: &crate::Db, route_id: &str, direction: i16) -> crate::Result<Self> {
    #[derive(sqlx::FromRow)]
    struct Row {
      direction_id: i16,
      trip_headsign: Option<String>,
    }

    let q = r#"
      SELECT DISTINCT ON (direction_id) direction_id, trip_headsign FROM trips
      WHERE route_id = $1 AND direction_id IS NOT NULL ORDER BY direction_id, trip_id
    "#;
    let directions = sqlx::query_as::<_, Row>(q).bind(route_id).fetch_all(db).await?;

    let available_directions: Vec<DirectionInfo> = directions
      .iter()
      .map(|r| DirectionInfo { direction_id: r.direction_id, headsign: r.trip_headsign.clone() })
      .collect();

    let headsign = directions.iter().find(|r| r.direction_id == direction).and_then(|r| r.trip_headsign.clone());

    let q = r#"
      SELECT ST_AsGeoJSON(sh.geom) FROM shapes sh JOIN trips  t  ON t.shape_id = sh.shape_id
      WHERE t.route_id = $1 AND t.direction_id = $2 ORDER BY t.trip_id LIMIT 1
    "#;
    let geojson_str: Option<String> = sqlx::query_scalar(q).bind(route_id).bind(direction).fetch_optional(db).await?;

    let shape_geojson = geojson_str
      .ok_or_else(|| crate::Error::NotFound(format!("no shape found for route {route_id} direction {direction}")))?;

    let shape: serde_json::Value = serde_json::from_str(&shape_geojson).unwrap_or(serde_json::Value::Null);

    let q = r#"
      SELECT s.stop_id, s.stop_name, st.stop_sequence, ST_Y(s.geom::geometry) AS lat, ST_X(s.geom::geometry) AS lng, s.bearing_rad
      FROM stop_times st JOIN stops s ON s.stop_id = st.stop_id
      WHERE st.trip_id = (SELECT trip_id FROM trips WHERE route_id = $1 AND direction_id = $2 ORDER BY trip_id LIMIT 1)
      ORDER BY st.stop_sequence
    "#;
    let stops = sqlx::query_as::<_, RouteStop>(q).bind(route_id).bind(direction).fetch_all(db).await?;

    Ok(RouteShape {
      route_id: route_id.to_string(),
      direction_id: direction,
      headsign,
      shape,
      stops,
      available_directions,
    })
  }
}

//- cache
use crate::cache::CacheRw;
impl Route {
  fn search_key(q: &str) -> String {
    format!("routes:search:{}", q.to_lowercase())
  }

  pub async fn cache_get_search(cache: &crate::Cache, q: &str) -> Option<Vec<Self>> {
    cache.get(&Self::search_key(q)).await
  }

  pub async fn cache_set_search(cache: &crate::Cache, q: &str, routes: &[Self]) {
    cache.set(&Self::search_key(q), &routes, cache.until_sync()).await;
  }
}

impl RouteShape {
  fn shape_key(line: &str, direction: i16) -> String {
    format!("routes:shape:{line}:{direction}")
  }

  pub async fn cache_get(cache: &crate::Cache, line: &str, direction: i16) -> Option<Self> {
    cache.get(&Self::shape_key(line, direction)).await
  }

  pub async fn cache_set(cache: &crate::Cache, line: &str, direction: i16, shape: &Self) {
    cache.set(&Self::shape_key(line, direction), shape, cache.until_sync()).await;
  }
}
