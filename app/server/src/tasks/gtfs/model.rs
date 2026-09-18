// https://api.montevideo.gub.uy/apidocs/publictransport#/gtfs
// https://gtfs.org/
use crate::Db;

use gtfs_structures::Gtfs;

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

pub(super) struct LocalGtfs;
impl LocalGtfs {
  pub async fn version(db: &Db) -> crate::Result<Option<String>> {
    let v = sqlx::query_scalar("SELECT version FROM gtfs_metadata WHERE id = 1").fetch_optional(db).await?;
    Ok(v)
  }

  pub async fn replace_transaction(db: &Db, gtfs: &Gtfs, new_version: &str) -> crate::Result<()> {
    let mut tx = db.begin().await?;
    Self::truncate_db(&mut tx).await?;

    Self::insert_stops(&mut tx, gtfs).await?;
    Self::insert_routes(&mut tx, gtfs).await?;
    Self::insert_shapes(&mut tx, gtfs).await?;
    Self::insert_calendar(&mut tx, gtfs).await?;
    Self::insert_trips(&mut tx, gtfs).await?;
    Self::insert_stop_times(&mut tx, gtfs).await?;

    Self::end(tx, gtfs, new_version).await?;

    Ok(())
  }

  async fn truncate_db(tx: &mut Tx<'_>) -> crate::Result<()> {
    let q = "TRUNCATE TABLE stop_times, trips, shapes, calendar, routes, stops RESTART IDENTITY CASCADE";
    sqlx::query(q).execute(tx.as_mut()).await?;
    Ok(())
  }

  async fn insert_stops(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    let len = gtfs.stops.len();
    tracing::info!("[GTFS] syncing {} stops", len);

    let mut stop_ids: Vec<String> = Vec::with_capacity(len);
    let mut stop_names: Vec<String> = Vec::with_capacity(len);
    let mut lngs: Vec<f64> = Vec::with_capacity(len);
    let mut lats: Vec<f64> = Vec::with_capacity(len);

    for (id, stop) in &gtfs.stops {
      let (lng, lat) = match (stop.longitude, stop.latitude) {
        (Some(lng), Some(lat)) if lng != 0.0 && lat != 0.0 => (lng, lat),
        _ => {
          tracing::warn!(stop_id = %id, "[GTFS] Stop has missing or zero coordinates, skipping");
          continue;
        }
      };

      stop_ids.push(id.clone());
      stop_names.push(stop.name.clone().unwrap_or_else(|| "Parada de bus".to_string()));
      lngs.push(lng);
      lats.push(lat);
    }

    let q = r#"
      INSERT INTO stops (stop_id, stop_name, geom)
      SELECT t.stop_id, t.stop_name, ST_SetSRID(ST_MakePoint(t.lng, t.lat), 4326)::geography
      FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::float8[]) AS t(stop_id, stop_name, lng, lat)
      "#;
    sqlx::query(q).bind(&stop_ids).bind(&stop_names).bind(&lngs).bind(&lats).execute(tx.as_mut()).await?;
    Ok(())
  }

  async fn insert_routes(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    let len = gtfs.routes.len();
    tracing::info!("[GTFS] syncing {} routes", len);

    let mut route_ids: Vec<String> = Vec::with_capacity(len);
    let mut short_names: Vec<String> = Vec::with_capacity(len);
    let mut long_names: Vec<Option<String>> = Vec::with_capacity(len);
    let mut agency_ids: Vec<Option<String>> = Vec::with_capacity(len);

    for (id, route) in &gtfs.routes {
      let short_name = match &route.short_name {
        Some(s) if !s.is_empty() => s.clone(),
        _ => {
          tracing::warn!(route_id = %id, "[GTFS] Route is missing short_name, skipping");
          continue;
        }
      };

      route_ids.push(id.clone());
      short_names.push(short_name);
      long_names.push(route.long_name.clone().filter(|s| !s.is_empty()));
      agency_ids.push(route.agency_id.clone());
    }

    let q = r#"
      INSERT INTO routes (route_id, route_short_name, route_long_name, agency_id)
      SELECT t.route_id, t.short_name, t.long_name, t.agency_id
      FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[]) AS t(route_id, short_name, long_name, agency_id)
      "#;
    sqlx::query(q).bind(&route_ids).bind(&short_names).bind(&long_names).bind(&agency_ids).execute(tx.as_mut()).await?;
    Ok(())
  }

  async fn insert_shapes(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    let len = gtfs.shapes.len();
    tracing::info!("[GTFS] syncing {} shapes", len);

    let mut shape_ids: Vec<String> = Vec::with_capacity(len);
    let mut wkts: Vec<String> = Vec::with_capacity(len);

    for (shape_id, points) in &gtfs.shapes {
      if points.is_empty() {
        tracing::warn!(shape_id = %shape_id, "[GTFS] Shape has no points, skipping");
        continue;
      }

      let mut sorted = points.clone();
      sorted.sort_unstable_by_key(|p| p.sequence);

      let valid: Vec<_> = sorted.iter().filter(|p| p.longitude != 0.0 && p.latitude != 0.0).collect();

      if valid.len() < 2 {
        tracing::warn!(shape_id = %shape_id, total = points.len(), "[GTFS] Shape has fewer than 2 valid points, skipping");
        continue;
      }

      let coords = valid.iter().map(|p| format!("{} {}", p.longitude, p.latitude)).collect::<Vec<_>>().join(",");
      shape_ids.push(shape_id.clone());
      wkts.push(format!("LINESTRING({coords})"));
    }

    let q = r#"
      INSERT INTO shapes (shape_id, geom)
      SELECT t.shape_id, ST_GeomFromText(t.wkt, 4326)
      FROM UNNEST($1::text[], $2::text[]) AS t(shape_id, wkt)
      "#;
    sqlx::query(q).bind(&shape_ids).bind(&wkts).execute(tx.as_mut()).await?;
    Ok(())
  }

  async fn insert_calendar(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    use chrono::NaiveDate;

    let len = gtfs.calendar.len();
    tracing::info!("[GTFS] syncing {} calendars", len);

    let mut service_ids: Vec<String> = Vec::with_capacity(len);
    let mut mondays: Vec<bool> = Vec::with_capacity(len);
    let mut tuesdays: Vec<bool> = Vec::with_capacity(len);
    let mut wednesdays: Vec<bool> = Vec::with_capacity(len);
    let mut thursdays: Vec<bool> = Vec::with_capacity(len);
    let mut fridays: Vec<bool> = Vec::with_capacity(len);
    let mut saturdays: Vec<bool> = Vec::with_capacity(len);
    let mut sundays: Vec<bool> = Vec::with_capacity(len);
    let mut start_dates: Vec<NaiveDate> = Vec::with_capacity(len);
    let mut end_dates: Vec<NaiveDate> = Vec::with_capacity(len);

    for (id, cal) in &gtfs.calendar {
      if cal.end_date < cal.start_date {
        tracing::warn!(service_id = %id, start = %cal.start_date, end = %cal.end_date, "[GTFS] Calendar entry has end_date before start_date, skipping");
        continue;
      }

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

    let q = r#"
      INSERT INTO calendar (service_id, monday, tuesday, wednesday, thursday, friday, saturday, sunday, start_date, end_date)
      SELECT t.service_id, t.mon, t.tue, t.wed, t.thu, t.fri, t.sat, t.sun, t.start_date, t.end_date
      FROM UNNEST($1::text[], $2::bool[], $3::bool[], $4::bool[], $5::bool[], $6::bool[], $7::bool[], $8::bool[], $9::date[], $10::date[])
      AS t(service_id, mon, tue, wed, thu, fri, sat, sun, start_date, end_date)
      "#;
    sqlx::query(q)
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
      .execute(tx.as_mut())
      .await?;
    Ok(())
  }

  async fn insert_trips(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    tracing::info!("[GTFS] syncing trips");

    const CHUNK: usize = 5_000;
    let trips_vec: Vec<(&String, &gtfs_structures::Trip)> = gtfs.trips.iter().collect();

    for chunk in trips_vec.chunks(CHUNK) {
      let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut route_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut service_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut shape_ids: Vec<Option<String>> = Vec::with_capacity(chunk.len());
      let mut direction_ids: Vec<Option<i16>> = Vec::with_capacity(chunk.len());
      let mut headsigns: Vec<Option<String>> = Vec::with_capacity(chunk.len());

      for (trip_id, trip) in chunk {
        if trip.route_id.is_empty() {
          tracing::warn!(trip_id = %trip_id, "[GTFS] Trip has empty route_id, skipping");
          continue;
        }
        if trip.service_id.is_empty() {
          tracing::warn!(trip_id = %trip_id, "[GTFS] Trip has empty service_id, skipping");
          continue;
        }

        trip_ids.push(trip_id.to_string());
        route_ids.push(trip.route_id.clone());
        service_ids.push(trip.service_id.clone());
        shape_ids.push(trip.shape_id.clone());
        direction_ids.push(trip.direction_id.as_ref().map(|d| match d {
          gtfs_structures::DirectionType::Outbound => 0,
          gtfs_structures::DirectionType::Inbound => 1,
        }));
        headsigns.push(trip.trip_headsign.clone());
      }

      let q = r#"
        INSERT INTO trips(trip_id, route_id, service_id, shape_id, direction_id, trip_headsign)
        SELECT t.trip_id, t.route_id, t.service_id, t.shape_id, t.direction_id, t.trip_headsign
        FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::int2[], $6::text[])
          AS t(trip_id, route_id, service_id, shape_id, direction_id, trip_headsign)
        ON CONFLICT (trip_id) DO NOTHING
        "#;
      sqlx::query(q)
        .bind(&trip_ids)
        .bind(&route_ids)
        .bind(&service_ids)
        .bind(&shape_ids)
        .bind(&direction_ids)
        .bind(&headsigns)
        .execute(tx.as_mut())
        .await?;
    }
    Ok(())
  }

  async fn insert_stop_times(tx: &mut Tx<'_>, gtfs: &Gtfs) -> crate::Result<()> {
    tracing::info!("[GTFS] syncing stop times");

    const CHUNK: usize = 5_000;

    let all_stop_times: Vec<(&str, &gtfs_structures::StopTime)> = gtfs
      .trips
      .iter()
      .flat_map(|(trip_id, trip)| trip.stop_times.iter().map(move |st| (trip_id.as_str(), st)))
      .collect();

    for chunk in all_stop_times.chunks(CHUNK) {
      let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut stop_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut sequences: Vec<i32> = Vec::with_capacity(chunk.len());
      let mut arrivals: Vec<Option<i32>> = Vec::with_capacity(chunk.len());
      let mut departures: Vec<Option<i32>> = Vec::with_capacity(chunk.len());

      for (trip_id, st) in chunk {
        if st.arrival_time.is_none() && st.departure_time.is_none() {
          tracing::warn!(trip_id = %trip_id, stop_id = %st.stop.id, sequence = st.stop_sequence, "[GTFS] Stop time has no arrival or departure, skipping");
          continue;
        }

        trip_ids.push(trip_id.to_string());
        stop_ids.push(st.stop.id.clone());
        sequences.push(st.stop_sequence as i32);
        arrivals.push(st.arrival_time.map(|t| t as i32));
        departures.push(st.departure_time.map(|t| t as i32));
      }

      let q = r#"
        INSERT INTO stop_times(trip_id, stop_id, stop_sequence, arrival_time, departure_time)
        SELECT t.trip_id, t.stop_id, t.stop_sequence,
          CASE WHEN t.arr IS NOT NULL THEN make_interval(secs => t.arr::float8) END,
          CASE WHEN t.dep IS NOT NULL THEN make_interval(secs => t.dep::float8) END
        FROM UNNEST($1::text[], $2::text[], $3::int4[], $4::int4[], $5::int4[])
          AS t(trip_id, stop_id, stop_sequence, arr, dep)
        ON CONFLICT (trip_id, stop_sequence) DO NOTHING
        "#;
      sqlx::query(q)
        .bind(&trip_ids)
        .bind(&stop_ids)
        .bind(&sequences)
        .bind(&arrivals)
        .bind(&departures)
        .execute(tx.as_mut())
        .await?;
    }
    Ok(())
  }

  async fn end(mut tx: Tx<'_>, gtfs: &Gtfs, version: &str) -> crate::Result<()> {
    let q = r#"
    INSERT INTO gtfs_metadata (id, version, synced_at) VALUES (1, $1, NOW())
    ON CONFLICT (id) DO UPDATE SET version = EXCLUDED.version, synced_at = EXCLUDED.synced_at
    "#;
    sqlx::query(q).bind(version).execute(tx.as_mut()).await?;

    tx.commit().await?;

    let n_stops = gtfs.stops.len();
    let n_routes = gtfs.routes.len();
    let n_shapes = gtfs.shapes.len();
    let n_trips = gtfs.trips.len();
    let n_stop_times: usize = gtfs.trips.values().map(|t| t.stop_times.len()).sum();
    tracing::info!(v = version, n_stops, n_routes, n_shapes, n_trips, n_stop_times, "[GTFS] sync complete");

    Ok(())
  }
}
