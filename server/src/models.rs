use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BusPosition {
    pub id: String,
    pub line: String,
    pub lat: f64,
    pub lng: f64,
    pub timestamp: DateTime<Utc>,
    pub origin: Option<String>,
    pub destination: Option<String>,
    pub company: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImBus {
    pub id: String,
    pub line: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub location: Option<ImGeoPoint>,
    pub company_name: Option<String>,
    pub origin: Option<String>,
    pub destination: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImGeoPoint {
    pub coordinates: Vec<f64>,
}

impl ImBus {
    pub fn into_position(self) -> Option<BusPosition> {
        let coords = self.location?.coordinates;
        if coords.len() < 2 {
            return None;
        }

        Some(BusPosition {
            id: self.id,
            line: self.line?,
            lng: coords[0],
            lat: coords[1],
            timestamp: self.timestamp?,
            origin: self.origin,
            destination: self.destination,
            company: self.company_name,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Route {
    pub route_id: String,
    pub route_short_name: String,
    pub route_long_name: Option<String>,
    pub agency_id: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NearbyStop {
    pub stop_id: String,
    pub stop_name: String,
    pub lat: f64,
    pub lng: f64,
    pub distance_meters: f64,
}

#[derive(Debug, Deserialize)]
pub struct NearbyStopsQuery {
    pub lat: f64,
    pub lng: f64,
    #[serde(default = "NearbyStopsQuery::default_radius")]
    pub radius: f64,
}

impl NearbyStopsQuery {
    fn default_radius() -> f64 {
        500.0
    }
}
