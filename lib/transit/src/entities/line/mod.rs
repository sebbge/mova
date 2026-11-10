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

///# GET <MOVA>/route/times/:provider/:route_id?direction_id=0&shape_id=&stop=
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteTimes {
  /// Next trip time per stop (s from midnight), `null` where it skips
  pub times: Vec<Option<i32>>,
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
  #[serde(skip_serializing_if = "Option::is_none")]
  pub area: Option<String>,
  pub stop_sequence: i32,
  pub lat: f64,
  pub lng: f64,
  pub bearing_rad: Option<f64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub station: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub platform: Option<String>,
}

/// Day type of a printed timetable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayType {
  #[serde(rename = "habil")]
  Workday,
  #[serde(rename = "sabado")]
  Saturday,
  #[serde(rename = "domingo")]
  Sunday,
}

///# GET <MOVA>/route/schedule/:provider/:route_id?direction_id=0&stop=<id>&day=habil|sabado|domingo
#[derive(Debug, Serialize, Deserialize)]
pub struct RouteSchedule {
  pub day: DayType,
  /// Today if it matches, else the next one
  pub date: String,
  /// By service hour
  pub hours: Vec<ScheduleHour>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleHour {
  /// 0-23
  pub hour: u32,
  /// Past 24 h in GTFS
  pub late: bool,
  pub departures: Vec<ScheduleDeparture>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ScheduleDeparture {
  /// s from service day midnight
  pub secs: i32,
  pub minute: i32,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub zonal: bool,
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  pub interpolated: bool,
  /// Next departure, today only
  #[serde(default, skip_serializing_if = "std::ops::Not::not")]
  #[sqlx(skip)]
  pub next: bool,
}
