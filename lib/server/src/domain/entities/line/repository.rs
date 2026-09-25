use super::{DirectionInfo, Route, RouteShape, RouteStop};
use crate::domain::scope::Scope;

//- sql
impl Route {
  pub async fn search(db: &crate::Db, region: &str, q: &str) -> crate::Result<Vec<Self>> {
    let pattern = format!("%{}%", q);

    let routes = sqlx::query_as::<_, Route>(
      r#"
    SELECT provider || '-' || route_id AS route_id, route_short_name, route_long_name, agency_id, area
    FROM routes
    WHERE region = $3
      AND (route_short_name ILIKE $1 OR route_long_name ILIKE $1)
    ORDER BY
      CASE WHEN UPPER(route_short_name) = UPPER($2) THEN 0 ELSE 1 END,
      route_short_name
    LIMIT 20
    "#,
    )
    .bind(&pattern)
    .bind(q)
    .bind(region)
    .fetch_all(db)
    .await?;

    Ok(routes)
  }
}

impl Route {
  pub async fn list(db: &crate::Db, region: &str) -> crate::Result<Vec<Self>> {
    let q = r#"
    SELECT provider || '-' || route_id AS route_id, route_short_name, route_long_name, agency_id, area
    FROM routes
    WHERE region = $1
    ORDER BY NULLIF(regexp_replace(route_short_name, '\D', '', 'g'), '')::bigint NULLS LAST,
             route_short_name, route_id
    "#;
    Ok(sqlx::query_as::<_, Route>(q).bind(region).fetch_all(db).await?)
  }

  pub async fn load_list(state: &crate::State, region: &str) -> crate::Result<Vec<Self>> {
    use crate::cache::CacheRw;
    let cache = &state.cache;
    cache.get_or_load(&crate::cache::key(region, "routes:all"), cache.until_sync(), Self::list(&state.db, region)).await
  }
}

struct Resolved {
  route_id: String,
  shape_id: String,
  direction_id: i16,
  headsign: Option<String>,
  available_directions: Vec<DirectionInfo>,
}

impl RouteShape {
  pub async fn get(db: &crate::Db, scope: &Scope, route_id: &str, direction: i16) -> crate::Result<Self> {
    let directions = Self::directions(db, scope, route_id).await?;
    let Some(first) = directions.first() else {
      return Err(crate::Error::NotFound(format!("no trips found for route {route_id}")));
    };

    // direction_id is a preference
    let resolved = directions.iter().find(|r| r.direction_id == direction).unwrap_or(first);
    let (direction, headsign) = (resolved.direction_id, resolved.headsign.clone());

    let q = r#"
      SELECT shape_id FROM trips
      WHERE region = $3 AND provider = $4 AND route_id = $1 AND direction_id = $2 AND shape_id IS NOT NULL
      ORDER BY trip_id LIMIT 1
    "#;
    let shape_id: Option<String> = sqlx::query_scalar(q)
      .bind(route_id)
      .bind(direction)
      .bind(&scope.region)
      .bind(&scope.provider)
      .fetch_optional(db)
      .await?;
    let shape_id = shape_id
      .ok_or_else(|| crate::Error::NotFound(format!("no shape found for route {route_id} direction {direction}")))?;

    let resolved = Resolved {
      route_id: route_id.to_string(),
      shape_id,
      direction_id: direction,
      headsign,
      available_directions: directions,
    };
    Self::build(db, scope, resolved).await
  }

  /// Route of a live bus, from its shape
  pub async fn get_by_shape(db: &crate::Db, scope: &Scope, shape_id: &str) -> crate::Result<Self> {
    #[derive(sqlx::FromRow)]
    struct Row {
      route_id: String,
      direction_id: i16,
      trip_headsign: Option<String>,
    }

    let q = r#"
      SELECT route_id, COALESCE(direction_id, 0)::int2 AS direction_id, trip_headsign
      FROM trips
      WHERE region = $2 AND provider = $3 AND shape_id = $1
      GROUP BY route_id, direction_id, trip_headsign
      ORDER BY COUNT(*) DESC
      LIMIT 1
    "#;
    let row = sqlx::query_as::<_, Row>(q)
      .bind(shape_id)
      .bind(&scope.region)
      .bind(&scope.provider)
      .fetch_optional(db)
      .await?
      .ok_or_else(|| crate::Error::NotFound(format!("no trips found for shape {shape_id}")))?;

    let available_directions = Self::directions(db, scope, &row.route_id).await?;
    let resolved = Resolved {
      route_id: row.route_id,
      shape_id: shape_id.to_string(),
      direction_id: row.direction_id,
      headsign: row.trip_headsign,
      available_directions,
    };
    Self::build(db, scope, resolved).await
  }

  // most frequent headsign per direction
  async fn directions(db: &crate::Db, scope: &Scope, route_id: &str) -> crate::Result<Vec<DirectionInfo>> {
    #[derive(sqlx::FromRow)]
    struct Row {
      direction_id: i16,
      trip_headsign: Option<String>,
    }

    let q = r#"
      SELECT DISTINCT ON (direction_id) direction_id, trip_headsign
      FROM (
        SELECT direction_id, trip_headsign, COUNT(*) AS trips
        FROM trips
        WHERE region = $2 AND provider = $3 AND route_id = $1 AND direction_id IS NOT NULL
        GROUP BY direction_id, trip_headsign
      ) t
      ORDER BY direction_id, trips DESC
    "#;
    let rows =
      sqlx::query_as::<_, Row>(q).bind(route_id).bind(&scope.region).bind(&scope.provider).fetch_all(db).await?;
    Ok(rows.into_iter().map(|r| DirectionInfo { direction_id: r.direction_id, headsign: r.trip_headsign }).collect())
  }

  async fn build(db: &crate::Db, scope: &Scope, r: Resolved) -> crate::Result<Self> {
    let q = r#"
      SELECT ST_AsGeoJSON(geom, 5) FROM shapes
      WHERE region = $2 AND provider = $3 AND shape_id = $1
    "#;
    let geojson_str: Option<String> =
      sqlx::query_scalar(q).bind(&r.shape_id).bind(&scope.region).bind(&scope.provider).fetch_optional(db).await?;

    let shape_geojson = geojson_str.ok_or_else(|| crate::Error::NotFound(format!("no shape {}", r.shape_id)))?;
    let shape: serde_json::Value = serde_json::from_str(&shape_geojson).unwrap_or(serde_json::Value::Null);

    let q = r#"
      SELECT s.provider || '-' || s.stop_id AS stop_id,
             COALESCE(s.cluster, s.provider || '-' || s.stop_id) AS cluster,
             s.stop_name, st.stop_sequence, round(ST_Y(s.geom::geometry)::numeric, 5)::float8 AS lat, round(ST_X(s.geom::geometry)::numeric, 5)::float8 AS lng, s.bearing_rad
      FROM stop_times st JOIN stops s ON s.region = st.region AND s.provider = st.provider AND s.stop_id = st.stop_id
      WHERE st.region = $3 AND st.provider = $4
        AND st.trip_id = (
          SELECT trip_id FROM trips
          WHERE region = $3 AND provider = $4 AND route_id = $1 AND shape_id = $2 ORDER BY trip_id LIMIT 1)
      ORDER BY st.stop_sequence
    "#;
    let stops = sqlx::query_as::<_, RouteStop>(q)
      .bind(&r.route_id)
      .bind(&r.shape_id)
      .bind(&scope.region)
      .bind(&scope.provider)
      .fetch_all(db)
      .await?;

    Ok(RouteShape {
      route_id: crate::domain::scope::qualify(&scope.provider, &r.route_id),
      shape_id: r.shape_id,
      direction_id: r.direction_id,
      headsign: r.headsign,
      shape,
      stops,
      available_directions: r.available_directions,
    })
  }
}

//- cache
use crate::cache::CacheRw;
impl Route {
  fn search_key(region: &str, q: &str) -> String {
    crate::cache::key(region, &format!("routes:search:{}", q.to_lowercase()))
  }

  pub async fn load_search(state: &crate::State, region: &str, q: &str) -> crate::Result<Vec<Self>> {
    let cache = &state.cache;
    cache.get_or_load(&Self::search_key(region, q), cache.until_sync(), Self::search(&state.db, region, q)).await
  }
}

impl RouteShape {
  fn shape_key(scope: &Scope, line: &str, direction: i16) -> String {
    crate::cache::key(&scope.region, &format!("routes:shape:{}:{line}:{direction}", scope.provider))
  }

  pub async fn load(state: &crate::State, scope: &Scope, route_id: &str, direction: i16) -> crate::Result<Self> {
    let cache = &state.cache;
    let key = Self::shape_key(scope, route_id, direction);
    cache.get_or_load(&key, cache.until_sync(), Self::get(&state.db, scope, route_id, direction)).await
  }

  pub async fn load_by_shape(state: &crate::State, scope: &Scope, shape_id: &str) -> crate::Result<Self> {
    let cache = &state.cache;
    let key = crate::cache::key(&scope.region, &format!("routes:shape_id:{}:{shape_id}", scope.provider));
    cache.get_or_load(&key, cache.until_sync(), Self::get_by_shape(&state.db, scope, shape_id)).await
  }
}
