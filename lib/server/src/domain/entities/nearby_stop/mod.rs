use serde::{Deserialize, Serialize};

mod repository;

///# GET <BONDI>/stops/nearby/?lat,lng,radius
#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct NearbyStop {
  pub stop_id: String,
  pub stop_name: String,
  pub lat: f64,
  pub lng: f64,
  pub distance_meters: f64,
  pub bearing_rad: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct NearbyStopQueryParams {
  pub lat: f64,
  pub lng: f64,
  #[serde(default = "default_radius")]
  pub radius: f64,
}
fn default_radius() -> f64 {
  1000.0
}
