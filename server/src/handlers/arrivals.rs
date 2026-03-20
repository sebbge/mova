use crate::AppState;
use crate::error::{AppError, AppResult};
use crate::models::ImBus;

use axum::Json;
use axum::extract::{Path, State};
use chrono::{Datelike, Duration, Local, Timelike, Weekday};
use futures::future::join_all;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Arrival {
    pub line: String,
    pub destination: Option<String>,
    pub origin: Option<String>,
    pub eta_minutes: Option<i64>,
    pub is_realtime: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImUpcomingBus {
    line: Option<String>,
    destination: Option<String>,
    origin: Option<String>,
}

pub async fn stop_arrivals(
    State(state): State<AppState>,
    Path(stop_id): Path<String>,
) -> AppResult<Json<Vec<Arrival>>> {
    let token = state.im_auth.token().await?;

    let upcoming_url = format!(
        "{}/buses/busstops/{}/upcomingbuses?amountperline=1",
        state.im_api_base_url, stop_id
    );

    let im_resp = state
        .http_client
        .get(&upcoming_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(e.to_string()))?;

    let upcoming: Vec<ImUpcomingBus> = if im_resp.status().is_success() {
        im_resp.json().await.unwrap_or_default()
    } else {
        vec![]
    };

    let mut seen = std::collections::HashSet::new();
    let lines: Vec<ImUpcomingBus> = upcoming
        .into_iter()
        .filter(|b| b.line.is_some())
        .filter(|b| seen.insert(b.line.clone().unwrap()))
        .collect();

    // realtime, else schedule
    let futures: Vec<_> = lines
        .into_iter()
        .map(|upcoming_bus| {
            let state = state.clone();
            let token = token.clone();
            let stop_id = stop_id.clone();
            async move {
                let line_name = upcoming_bus.line.unwrap();
                let eta = realtime_eta(&state, &token, &line_name, &stop_id).await;

                match eta {
                    Some(mins) => Arrival {
                        line: line_name,
                        destination: upcoming_bus.destination,
                        origin: upcoming_bus.origin,
                        eta_minutes: Some(mins),
                        is_realtime: true,
                    },
                    None => Arrival {
                        eta_minutes: schedule_eta(&state, &line_name, &stop_id).await,
                        line: line_name,
                        destination: upcoming_bus.destination,
                        origin: upcoming_bus.origin,
                        is_realtime: false,
                    },
                }
            }
        })
        .collect();

    let mut arrivals = join_all(futures).await;

    if arrivals.is_empty() {
        arrivals = gtfs_only_arrivals(&state, &stop_id).await?;
    }

    arrivals.sort_by_key(|a| (a.eta_minutes.is_none(), a.eta_minutes));

    Ok(Json(arrivals))
}

async fn realtime_eta(
    state: &AppState,
    token: &str,
    line_name: &str,
    stop_id: &str,
) -> Option<i64> {
    let url = format!(
        "{}/buses?lines={}&busstopId={}",
        state.im_api_base_url, line_name, stop_id
    );

    let resp = state
        .http_client
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let buses: Vec<ImBus> = resp.json().await.ok()?;
    let positions: Vec<_> = buses
        .into_iter()
        .filter_map(|b| b.into_position())
        .collect();

    if positions.is_empty() {
        return None;
    }

    let stop_coords: Option<(f64, f64)> = sqlx::query_as::<_, (f64, f64)>(
        "SELECT ST_Y(geom::geometry), ST_X(geom::geometry) FROM stops WHERE stop_id = $1",
    )
    .bind(stop_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten();

    let (stop_lat, stop_lng) = stop_coords?;

    let closest = positions.iter().min_by(|a, b| {
        let da = haversine(a.lat, a.lng, stop_lat, stop_lng);
        let db = haversine(b.lat, b.lng, stop_lat, stop_lng);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    })?;

    // 20 km/h
    let dist_km = haversine(closest.lat, closest.lng, stop_lat, stop_lng);
    let eta_mins = (dist_km / 20.0 * 60.0).round() as i64;

    if eta_mins > 90 {
        return None;
    }

    Some(eta_mins)
}

async fn schedule_eta(state: &AppState, line_name: &str, stop_id: &str) -> Option<i64> {
    let now = Local::now();
    let today_secs = now.num_seconds_from_midnight() as i64;
    let today_interval = format!("{} seconds", today_secs.max(0));

    let day_col = calendar_column(now.weekday());

    let query = format!(
        r#"
        SELECT EXTRACT(EPOCH FROM (st.departure_time - $1::interval))::bigint AS secs
        FROM stop_times st
        JOIN trips t ON t.trip_id = st.trip_id
        JOIN routes r ON r.route_id = t.route_id
        JOIN calendar c ON c.service_id = t.service_id
        WHERE r.route_short_name = $2
          AND st.stop_id = $3
          AND st.departure_time > $1::interval
          AND c.{day_col} = true
          AND c.start_date <= $4::date
          AND c.end_date >= $4::date
        ORDER BY st.departure_time
        LIMIT 1
        "#
    );

    let secs: i64 = sqlx::query_scalar(&query)
        .bind(&today_interval)
        .bind(line_name)
        .bind(stop_id)
        .bind(now.date_naive())
        .fetch_optional(&state.db)
        .await
        .ok()??;

    Some(Duration::seconds(secs).num_minutes())
}

async fn gtfs_only_arrivals(state: &AppState, stop_id: &str) -> AppResult<Vec<Arrival>> {
    let now = Local::now();
    let today_secs = now.num_seconds_from_midnight() as i64;
    let today_interval = format!("{} seconds", today_secs);

    let day_col = calendar_column(now.weekday());

    let query = format!(
        r#"
        SELECT DISTINCT ON (r.route_short_name)
            r.route_short_name AS line,
            t.trip_headsign    AS destination,
            EXTRACT(EPOCH FROM (st.departure_time - $1::interval))::bigint AS secs
        FROM stop_times st
        JOIN trips t ON t.trip_id = st.trip_id
        JOIN routes r ON r.route_id = t.route_id
        JOIN calendar c ON c.service_id = t.service_id
        WHERE st.stop_id = $2
          AND st.departure_time > $1::interval
          AND c.{day_col} = true
          AND c.start_date <= $3::date
          AND c.end_date >= $3::date
        ORDER BY r.route_short_name, st.departure_time
        LIMIT 20
        "#
    );

    #[derive(sqlx::FromRow)]
    struct Row {
        line: String,
        destination: Option<String>,
        secs: Option<i64>,
    }

    let rows: Vec<Row> = sqlx::query_as(&query)
        .bind(&today_interval)
        .bind(stop_id)
        .bind(now.date_naive())
        .fetch_all(&state.db)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| Arrival {
            line: r.line,
            destination: r.destination,
            origin: None,
            eta_minutes: r.secs.map(|s| Duration::seconds(s).num_minutes()),
            is_realtime: false,
        })
        .collect())
}

fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    R * 2.0 * a.sqrt().asin()
}

fn calendar_column(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
}
