use crate::AppState;
use crate::error::{AppError, AppResult};
use crate::models::Route;

use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<Vec<Route>>> {
    let pattern = format!("%{}%", params.q);

    let routes = sqlx::query_as::<_, Route>(
        r#"
        SELECT route_id, route_short_name, route_long_name, agency_id
        FROM routes
        WHERE route_short_name ILIKE $1
           OR route_long_name  ILIKE $1
        ORDER BY route_short_name
        LIMIT 20
        "#,
    )
    .bind(&pattern)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(routes))
}

#[derive(Debug, Deserialize)]
pub struct ShapeQuery {
    /// 0 outbound, 1 inbound
    pub direction_id: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct DirectionInfo {
    pub direction_id: i16,
    pub headsign: Option<String>,
}

#[derive(sqlx::FromRow)]
struct DirectionRow {
    direction_id: i16,
    trip_headsign: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RouteShapeResponse {
    pub route_id: String,
    pub direction_id: i16,
    pub headsign: Option<String>,
    /// GeoJSON LineString
    pub shape: serde_json::Value,
    pub stops: Vec<RouteStop>,
    pub available_directions: Vec<DirectionInfo>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RouteStop {
    pub stop_id: String,
    pub stop_name: String,
    pub stop_sequence: i32,
    pub lat: f64,
    pub lng: f64,
}

pub async fn shape(
    State(state): State<AppState>,
    Path(route_id): Path<String>,
    Query(params): Query<ShapeQuery>,
) -> AppResult<Json<RouteShapeResponse>> {
    let direction = params.direction_id.unwrap_or(0);

    let direction_rows = sqlx::query_as::<_, DirectionRow>(
        r#"
        SELECT DISTINCT ON (direction_id)
            direction_id,
            trip_headsign
        FROM trips
        WHERE route_id = $1
          AND direction_id IS NOT NULL
        ORDER BY direction_id, trip_id
        "#,
    )
    .bind(&route_id)
    .fetch_all(&state.db)
    .await?;

    let available_directions: Vec<DirectionInfo> = direction_rows
        .iter()
        .map(|r| DirectionInfo {
            direction_id: r.direction_id,
            headsign: r.trip_headsign.clone(),
        })
        .collect();

    let headsign = direction_rows
        .iter()
        .find(|r| r.direction_id == direction)
        .and_then(|r| r.trip_headsign.clone());

    // first trip's shape
    let geojson_str: Option<String> = sqlx::query_scalar(
        r#"
        SELECT ST_AsGeoJSON(sh.geom)
        FROM shapes sh
        JOIN trips  t  ON t.shape_id = sh.shape_id
        WHERE t.route_id    = $1
          AND t.direction_id = $2
        ORDER BY t.trip_id
        LIMIT 1
        "#,
    )
    .bind(&route_id)
    .bind(direction)
    .fetch_optional(&state.db)
    .await?;

    let shape_geojson = geojson_str.ok_or_else(|| {
        AppError::NotFound(format!(
            "no shape found for route {route_id} direction {direction}"
        ))
    })?;

    let shape: serde_json::Value =
        serde_json::from_str(&shape_geojson).unwrap_or(serde_json::Value::Null);

    // first trip's stops
    let stops = sqlx::query_as::<_, RouteStop>(
        r#"
        SELECT
            s.stop_id,
            s.stop_name,
            st.stop_sequence,
            ST_Y(s.geom::geometry) AS lat,
            ST_X(s.geom::geometry) AS lng
        FROM stop_times st
        JOIN stops s ON s.stop_id = st.stop_id
        WHERE st.trip_id = (
            SELECT trip_id FROM trips
            WHERE route_id    = $1
              AND direction_id = $2
            ORDER BY trip_id
            LIMIT 1
        )
        ORDER BY st.stop_sequence
        "#,
    )
    .bind(&route_id)
    .bind(direction)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(RouteShapeResponse {
        route_id,
        direction_id: direction,
        headsign,
        shape,
        stops,
        available_directions,
    }))
}
