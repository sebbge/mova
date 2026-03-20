use crate::AppState;
use crate::error::AppError;

use gtfs_structures::Gtfs;
use std::io::Cursor;

const CHUNK: usize = 5_000;

#[derive(Debug)]
#[allow(dead_code)]
pub struct SyncResult {
    pub version: String,
    pub status: &'static str,
    pub stops: usize,
    pub routes: usize,
    pub shapes: usize,
    pub trips: usize,
    pub stop_times: usize,
}

// version.txt is unreliable upstream
pub async fn fetch(state: &AppState) -> Result<(reqwest::Response, String), AppError> {
    let zip_url = format!(
        "{}/buses/gtfs/static/latest/google_transit.zip",
        state.im_api_base_url
    );

    let token = state.im_auth.token().await?;
    let resp = state
        .http_client
        .get(&zip_url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| AppError::ExternalApi(format!("GTFS request failed: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::ExternalApi(format!(
            "GTFS returned HTTP {}",
            resp.status()
        )));
    }

    let version = resp
        .headers()
        .get("etag")
        .or_else(|| resp.headers().get("last-modified"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"').to_string())
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());

    Ok((resp, version))
}

pub async fn check_and_sync(state: &AppState) -> Result<SyncResult, AppError> {
    let (resp, version) = fetch(state).await?;

    let current: Option<String> =
        sqlx::query_scalar("SELECT version FROM gtfs_metadata WHERE id = 1")
            .fetch_optional(&state.db)
            .await?;

    if current.as_deref() == Some(version.as_str()) {
        tracing::info!(version, "GTFS already up to date — skipping download");
        return Ok(SyncResult {
            version,
            status: "up_to_date",
            stops: 0,
            routes: 0,
            shapes: 0,
            trips: 0,
            stop_times: 0,
        });
    }

    tracing::info!(
        version,
        previous = ?current,
        "GTFS version changed — starting sync"
    );

    let zip_bytes = resp
        .bytes()
        .await
        .map_err(|e| AppError::ExternalApi(format!("ZIP read failed: {e}")))?;

    let gtfs = tokio::task::spawn_blocking(move || Gtfs::from_reader(Cursor::new(zip_bytes)))
        .await
        .map_err(|e| AppError::ExternalApi(format!("GTFS parse task panicked: {e}")))?
        .map_err(|e| AppError::ExternalApi(format!("GTFS parse failed: {e}")))?;

    let n_stops = gtfs.stops.len();
    let n_routes = gtfs.routes.len();
    let n_shapes = gtfs.shapes.len();
    let n_trips = gtfs.trips.len();
    let n_stop_times: usize = gtfs.trips.values().map(|t| t.stop_times.len()).sum();

    let mut tx = state.db.begin().await?;

    sqlx::query(
        "TRUNCATE TABLE stop_times, trips, shapes, calendar, routes, stops RESTART IDENTITY CASCADE",
    )
    .execute(&mut *tx)
    .await?;

    // stops
    {
        let mut stop_ids: Vec<String> = Vec::with_capacity(n_stops);
        let mut stop_names: Vec<String> = Vec::with_capacity(n_stops);
        let mut lngs: Vec<f64> = Vec::with_capacity(n_stops);
        let mut lats: Vec<f64> = Vec::with_capacity(n_stops);

        for (id, stop) in &gtfs.stops {
            stop_ids.push(id.clone());
            stop_names.push(stop.name.clone().unwrap_or_default());
            lngs.push(stop.longitude.unwrap_or(0.0));
            lats.push(stop.latitude.unwrap_or(0.0));
        }

        sqlx::query(
            r#"
            INSERT INTO stops (stop_id, stop_name, geom)
            SELECT t.stop_id, t.stop_name,
                   ST_SetSRID(ST_MakePoint(t.lng, t.lat), 4326)::geography
            FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::float8[])
              AS t(stop_id, stop_name, lng, lat)
            "#,
        )
        .bind(&stop_ids)
        .bind(&stop_names)
        .bind(&lngs)
        .bind(&lats)
        .execute(&mut *tx)
        .await?;
    }

    // routes
    {
        let mut route_ids: Vec<String> = Vec::with_capacity(n_routes);
        let mut short_names: Vec<String> = Vec::with_capacity(n_routes);
        let mut long_names: Vec<Option<String>> = Vec::with_capacity(n_routes);
        let mut agency_ids: Vec<Option<String>> = Vec::with_capacity(n_routes);

        for (id, route) in &gtfs.routes {
            route_ids.push(id.clone());
            short_names.push(route.short_name.clone().unwrap_or_default());
            long_names.push(route.long_name.clone().filter(|s| !s.is_empty()));
            agency_ids.push(route.agency_id.clone());
        }

        sqlx::query(
            r#"
            INSERT INTO routes (route_id, route_short_name, route_long_name, agency_id)
            SELECT t.route_id, t.short_name, t.long_name, t.agency_id
            FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[])
              AS t(route_id, short_name, long_name, agency_id)
            "#,
        )
        .bind(&route_ids)
        .bind(&short_names)
        .bind(&long_names)
        .bind(&agency_ids)
        .execute(&mut *tx)
        .await?;
    }

    // shapes as wkt linestrings
    {
        let mut shape_ids: Vec<String> = Vec::with_capacity(n_shapes);
        let mut wkts: Vec<String> = Vec::with_capacity(n_shapes);

        for (shape_id, points) in &gtfs.shapes {
            if points.is_empty() {
                continue;
            }
            let mut sorted = points.clone();
            sorted.sort_unstable_by_key(|p| p.sequence);

            let coords = sorted
                .iter()
                .map(|p| format!("{} {}", p.longitude, p.latitude))
                .collect::<Vec<_>>()
                .join(",");

            shape_ids.push(shape_id.clone());
            wkts.push(format!("LINESTRING({coords})"));
        }

        sqlx::query(
            r#"
            INSERT INTO shapes (shape_id, geom)
            SELECT t.shape_id, ST_GeomFromText(t.wkt, 4326)
            FROM UNNEST($1::text[], $2::text[]) AS t(shape_id, wkt)
            "#,
        )
        .bind(&shape_ids)
        .bind(&wkts)
        .execute(&mut *tx)
        .await?;
    }

    // calendar
    {
        use chrono::NaiveDate;

        let mut service_ids: Vec<String> = Vec::new();
        let mut mondays: Vec<bool> = Vec::new();
        let mut tuesdays: Vec<bool> = Vec::new();
        let mut wednesdays: Vec<bool> = Vec::new();
        let mut thursdays: Vec<bool> = Vec::new();
        let mut fridays: Vec<bool> = Vec::new();
        let mut saturdays: Vec<bool> = Vec::new();
        let mut sundays: Vec<bool> = Vec::new();
        let mut start_dates: Vec<NaiveDate> = Vec::new();
        let mut end_dates: Vec<NaiveDate> = Vec::new();

        for (id, cal) in &gtfs.calendar {
            service_ids.push(id.clone());
            mondays.push(cal.monday);
            tuesdays.push(cal.tuesday);
            wednesdays.push(cal.wednesday);
            thursdays.push(cal.thursday);
            fridays.push(cal.friday);
            saturdays.push(cal.saturday);
            sundays.push(cal.sunday);
            start_dates.push(cal.start_date);
            end_dates.push(cal.end_date);
        }

        sqlx::query(
            r#"
            INSERT INTO calendar (
                service_id, monday, tuesday, wednesday, thursday,
                friday, saturday, sunday, start_date, end_date
            )
            SELECT t.service_id, t.mon, t.tue, t.wed, t.thu,
                   t.fri, t.sat, t.sun, t.start_date, t.end_date
            FROM UNNEST(
                $1::text[], $2::bool[], $3::bool[], $4::bool[], $5::bool[],
                $6::bool[], $7::bool[], $8::bool[], $9::date[], $10::date[]
            ) AS t(service_id, mon, tue, wed, thu, fri, sat, sun, start_date, end_date)
            "#,
        )
        .bind(&service_ids)
        .bind(&mondays)
        .bind(&tuesdays)
        .bind(&wednesdays)
        .bind(&thursdays)
        .bind(&fridays)
        .bind(&saturdays)
        .bind(&sundays)
        .bind(&start_dates)
        .bind(&end_dates)
        .execute(&mut *tx)
        .await?;
    }

    // trips
    {
        let trips_vec: Vec<(&String, &gtfs_structures::Trip)> = gtfs.trips.iter().collect();

        for chunk in trips_vec.chunks(CHUNK) {
            let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut route_ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut service_ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut shape_ids: Vec<Option<String>> = Vec::with_capacity(chunk.len());
            let mut direction_ids: Vec<Option<i16>> = Vec::with_capacity(chunk.len());
            let mut headsigns: Vec<Option<String>> = Vec::with_capacity(chunk.len());

            for (trip_id, trip) in chunk {
                trip_ids.push(trip_id.to_string());
                route_ids.push(trip.route_id.clone());
                service_ids.push(trip.service_id.clone());
                shape_ids.push(trip.shape_id.clone());
                direction_ids.push(trip.direction_id.as_ref().map(|d| match d {
                    gtfs_structures::DirectionType::Outbound => 0i16,
                    gtfs_structures::DirectionType::Inbound => 1i16,
                }));
                headsigns.push(trip.trip_headsign.clone());
            }

            sqlx::query(
                r#"
                INSERT INTO trips
                    (trip_id, route_id, service_id, shape_id, direction_id, trip_headsign)
                SELECT t.trip_id, t.route_id, t.service_id, t.shape_id,
                       t.direction_id, t.trip_headsign
                FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::int2[], $6::text[])
                  AS t(trip_id, route_id, service_id, shape_id, direction_id, trip_headsign)
                ON CONFLICT (trip_id) DO NOTHING
                "#,
            )
            .bind(&trip_ids)
            .bind(&route_ids)
            .bind(&service_ids)
            .bind(&shape_ids)
            .bind(&direction_ids)
            .bind(&headsigns)
            .execute(&mut *tx)
            .await?;
        }
    }

    // stop times
    {
        let all_stop_times: Vec<(&str, &gtfs_structures::StopTime)> = gtfs
            .trips
            .iter()
            .flat_map(|(trip_id, trip)| {
                trip.stop_times.iter().map(move |st| (trip_id.as_str(), st))
            })
            .collect();

        for chunk in all_stop_times.chunks(CHUNK) {
            let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut stop_ids: Vec<String> = Vec::with_capacity(chunk.len());
            let mut sequences: Vec<i32> = Vec::with_capacity(chunk.len());
            let mut arrivals: Vec<Option<i32>> = Vec::with_capacity(chunk.len());
            let mut departures: Vec<Option<i32>> = Vec::with_capacity(chunk.len());

            for (trip_id, st) in chunk {
                trip_ids.push(trip_id.to_string());
                stop_ids.push(st.stop.id.clone());
                sequences.push(st.stop_sequence as i32);
                arrivals.push(st.arrival_time.map(|t| t as i32));
                departures.push(st.departure_time.map(|t| t as i32));
            }

            sqlx::query(
                r#"
                INSERT INTO stop_times
                    (trip_id, stop_id, stop_sequence, arrival_time, departure_time)
                SELECT
                    t.trip_id, t.stop_id, t.stop_sequence,
                    CASE WHEN t.arr IS NOT NULL
                         THEN make_interval(secs => t.arr::float8) END,
                    CASE WHEN t.dep IS NOT NULL
                         THEN make_interval(secs => t.dep::float8) END
                FROM UNNEST($1::text[], $2::text[], $3::int4[], $4::int4[], $5::int4[])
                  AS t(trip_id, stop_id, stop_sequence, arr, dep)
                ON CONFLICT (trip_id, stop_sequence) DO NOTHING
                "#,
            )
            .bind(&trip_ids)
            .bind(&stop_ids)
            .bind(&sequences)
            .bind(&arrivals)
            .bind(&departures)
            .execute(&mut *tx)
            .await?;
        }
    }

    // metadata
    sqlx::query(
        r#"
        INSERT INTO gtfs_metadata (id, version, synced_at)
        VALUES (1, $1, NOW())
        ON CONFLICT (id) DO UPDATE
            SET version   = EXCLUDED.version,
                synced_at = EXCLUDED.synced_at
        "#,
    )
    .bind(&version)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(
        version,
        n_stops,
        n_routes,
        n_shapes,
        n_trips,
        n_stop_times,
        "GTFS sync complete"
    );

    Ok(SyncResult {
        version,
        status: "synced",
        stops: n_stops,
        routes: n_routes,
        shapes: n_shapes,
        trips: n_trips,
        stop_times: n_stop_times,
    })
}

/// Sync now, then daily at 02:00 local
pub fn spawn_scheduler(state: AppState) {
    tokio::spawn(async move {
        run_once(&state).await;
        loop {
            let wait = duration_until_2am();
            tracing::info!("GTFS scheduler: next version check in {wait:?}");
            tokio::time::sleep(wait).await;
            run_once(&state).await;
        }
    });
}

async fn run_once(state: &AppState) {
    match check_and_sync(state).await {
        Ok(r) => tracing::info!(
            status = r.status,
            version = r.version,
            stops = r.stops,
            routes = r.routes,
            "GTFS check complete"
        ),
        Err(e) => tracing::error!("GTFS sync failed: {e:?}"),
    }
}

fn duration_until_2am() -> std::time::Duration {
    use chrono::{Local, Timelike};

    let now = Local::now();
    let now_secs = now.time().num_seconds_from_midnight() as i64;
    let target_secs: i64 = 2 * 3600;

    let remaining = if target_secs > now_secs {
        target_secs - now_secs
    } else {
        86_400 - now_secs + target_secs
    };

    std::time::Duration::from_secs(remaining as u64)
}
