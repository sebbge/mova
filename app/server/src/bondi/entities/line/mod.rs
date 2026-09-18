use serde::{Deserialize, Serialize};

mod repository;

///# GET <BONDI>/routes/search?q={term}
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Route {
  pub route_id: String,
  pub route_short_name: String,
  pub route_long_name: Option<String>,
  pub agency_id: Option<String>, // always 'STM-MVD'
}

///# GET <BONDI>/routes/:route_id/shape?direction_id=0|1
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteShape {
  pub route_id: String,
  pub direction_id: i16,
  pub headsign: Option<String>,
  pub shape: serde_json::Value,
  pub stops: Vec<RouteStop>,
  pub available_directions: Vec<DirectionInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectionInfo {
  pub direction_id: i16,
  pub headsign: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct RouteStop {
  pub stop_id: String,
  pub stop_name: String,
  pub stop_sequence: i32,
  pub lat: f64,
  pub lng: f64,
  pub bearing_rad: Option<f64>,
}
