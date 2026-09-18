use super::{NearbyStop, NearbyStopQueryParams};

// sql
impl NearbyStop {
  pub async fn get(db: &crate::Db, p: &NearbyStopQueryParams) -> crate::Result<Vec<Self>> {
    let q = r#"
    SELECT s.stop_id, s.stop_name, ST_Y(s.geom::geometry)  AS lat, ST_X(s.geom::geometry)  AS lng, s.bearing_rad,
    ST_Distance(s.geom, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography) AS distance_meters
    FROM stops s WHERE ST_DWithin(s.geom, ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography, $3)
    ORDER BY distance_meters LIMIT 50
    "#;
    let stops = sqlx::query_as::<_, NearbyStop>(q).bind(p.lng).bind(p.lat).bind(p.radius).fetch_all(db).await?;
    Ok(stops)
  }
}

// cache
use crate::cache::CacheRw;
impl NearbyStop {
  fn nearby_key(lat: f64, lng: f64, radius: f64) -> String {
    format!("stops:nearby:{lat:.3}:{lng:.3}:{radius:.0}")
  }

  pub async fn cache_get(cache: &crate::Cache, p: &NearbyStopQueryParams) -> Option<Vec<Self>> {
    cache.get(&Self::nearby_key(p.lat, p.lng, p.radius)).await
  }

  pub async fn cache_set(cache: &crate::Cache, p: &NearbyStopQueryParams, stops: &[Self]) {
    cache.set(&Self::nearby_key(p.lat, p.lng, p.radius), &stops, cache.until_sync()).await;
  }
}
