use crate::AppState;
use crate::error::AppResult;
use crate::models::{NearbyStop, NearbyStopsQuery};

use axum::Json;
use axum::extract::{Query, State};

pub async fn nearby(
    State(state): State<AppState>,
    Query(params): Query<NearbyStopsQuery>,
) -> AppResult<Json<Vec<NearbyStop>>> {
    let stops = sqlx::query_as::<_, NearbyStop>(
        r#"
        SELECT
            s.stop_id,
            s.stop_name,
            ST_Y(s.geom::geometry)  AS lat,
            ST_X(s.geom::geometry)  AS lng,
            ST_Distance(
                s.geom,
                ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography
            ) AS distance_meters
        FROM stops s
        WHERE ST_DWithin(
            s.geom,
            ST_SetSRID(ST_MakePoint($1, $2), 4326)::geography,
            $3
        )
        ORDER BY distance_meters
        LIMIT 50
        "#,
    )
    .bind(params.lng)
    .bind(params.lat)
    .bind(params.radius)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(stops))
}
