use serde::{Deserialize, Serialize};

mod repository;

///# GET <MOVA>/routes/search?q={term}
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Route {
  pub route_id: String,
  pub route_short_name: String,
  pub route_long_name: Option<String>,
  pub agency_id: Option<String>,
  pub region: String,
  /// Department with most of its stops
  pub area: Option<String>,
}

///# GET <MOVA>/route/shape/:provider/:route_id?direction_id=0|1
///# GET <MOVA>/shape/:provider/:shape_id
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteShape {
  pub route_id: String,
  /// Drawn shape
  pub shape_id: Option<String>,
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
  /// Shelter ref
  pub cluster: String,
  pub stop_name: String,
  pub stop_sequence: i32,
  pub lat: f64,
  pub lng: f64,
  pub bearing_rad: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub station: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub platform: Option<String>,
}
