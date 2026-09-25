// https://api.montevideo.gub.uy/apidocs/publictransport#/gtfs
// https://gtfs.org/
use crate::Db;
use crate::core::provider::Feed;

use gtfs_structures::Gtfs;
use std::collections::HashSet;

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

/// Loaded ids, refs checked here (no FK on trips, stop_times)
#[derive(Default)]
struct Loaded<'g> {
  stops: HashSet<&'g str>,
  routes: HashSet<&'g str>,
  shapes: HashSet<&'g str>,
  services: HashSet<&'g str>,
  trips: HashSet<&'g str>,
}

pub(super) struct LocalGtfs;
impl LocalGtfs {
  pub async fn version(db: &Db, provider: &str) -> crate::Result<Option<String>> {
    let v = sqlx::query_scalar("SELECT version FROM gtfs_metadata WHERE provider = $1")
      .bind(provider)
      .fetch_optional(db)
      .await?;
    Ok(v)
  }

  pub async fn replace_transaction(
    db: &Db,
    gtfs: &Gtfs,
    new_version: &str,
    provider: &str,
    region: &str,
    names: &dyn Feed,
    lines_by_stop: Option<&crate::core::provider::LinesByStop>,
  ) -> crate::Result<()> {
    let mut tx = db.begin().await?;
    Self::clear_provider(&mut tx, provider).await?;

    let mut loaded = Loaded::default();
    Self::insert_stops(&mut tx, gtfs, provider, region, names, &mut loaded).await?;
    Self::insert_routes(&mut tx, gtfs, provider, region, names, &mut loaded).await?;
    Self::insert_shapes(&mut tx, gtfs, provider, region, &mut loaded).await?;
    Self::insert_calendar(&mut tx, gtfs, provider, region, &mut loaded).await?;
    Self::insert_trips(&mut tx, gtfs, provider, region, names, &mut loaded).await?;
    Self::insert_stop_times(&mut tx, gtfs, provider, region, &loaded).await?;

    Self::analyze(&mut tx).await?;
    Self::resolve_route_areas(&mut tx, provider).await?;

    if let Some(lines_by_stop) = lines_by_stop {
      Self::insert_stop_lines(&mut tx, lines_by_stop, provider, region).await?;
      Self::mark_zonal(&mut tx, provider, region).await?;
    }

    Self::end(tx, gtfs, new_version, provider).await?;

    Ok(())
  }

  /// Stats for the rows of this transaction
  async fn analyze(tx: &mut Tx<'_>) -> crate::Result<()> {
    for table in ["stops", "routes", "shapes", "calendar", "trips", "stop_times"] {
      let analyze = format!("ANALYZE {table}");
      sqlx::query(sqlx::AssertSqlSafe(analyze)).execute(tx.as_mut()).await?;
    }
    Ok(())
  }

  /// This provider only
  async fn clear_provider(tx: &mut Tx<'_>, provider: &str) -> crate::Result<()> {
    for q in [
      "DELETE FROM stop_lines WHERE provider = $1",
      "DELETE FROM stop_times WHERE provider = $1",
      "DELETE FROM trips WHERE provider = $1",
      "DELETE FROM shapes WHERE provider = $1",
      "DELETE FROM calendar WHERE provider = $1",
      "DELETE FROM routes WHERE provider = $1",
      "DELETE FROM stops WHERE provider = $1",
    ] {
      sqlx::query(q).bind(provider).execute(tx.as_mut()).await?;
    }
    Ok(())
  }

  /// Department with most of its stops
  async fn resolve_route_areas(tx: &mut Tx<'_>, provider: &str) -> crate::Result<()> {
    let q = r#"
      WITH single AS MATERIALIZED (
        SELECT DISTINCT ON (region, route_id, shape_id) region, provider, route_id, trip_id
        FROM trips WHERE provider = $1
        ORDER BY region, route_id, shape_id, trip_id
      ),
      passes AS MATERIALIZED (
        SELECT DISTINCT u.region, u.provider, u.route_id, st.stop_id
        FROM single u
        JOIN stop_times st ON st.region = u.region AND st.provider = u.provider AND st.trip_id = u.trip_id
      )
      UPDATE routes r SET area = d.area
      FROM (
        SELECT DISTINCT ON (p.region, p.route_id) p.region, p.route_id, s.area
        FROM passes p
        JOIN stops s ON s.region = p.region AND s.provider = p.provider AND s.stop_id = p.stop_id
        WHERE s.area IS NOT NULL
        GROUP BY p.region, p.route_id, s.area
        ORDER BY p.region, p.route_id, count(*) DESC, s.area
      ) d
      WHERE r.provider = $1 AND r.region = d.region AND r.route_id = d.route_id
      "#;
    let done = sqlx::query(q).bind(provider).execute(tx.as_mut()).await?;
    tracing::info!(rows = done.rows_affected(), "[gtfs] route areas resolved");
    Ok(())
  }

  /// Marks zone markers, before their schedules move
  async fn mark_zonal(tx: &mut Tx<'_>, provider: &str, region: &str) -> crate::Result<()> {
    let q = r#"
    UPDATE stops s SET zonal = TRUE
    WHERE s.region = $2 AND s.provider = $1
      AND EXISTS (SELECT 1 FROM stop_times t
                  WHERE t.region = s.region AND t.provider = s.provider AND t.stop_id = s.stop_id)
      AND NOT EXISTS (SELECT 1 FROM stop_lines l
                      WHERE l.region = s.region AND l.provider = s.provider AND l.stop_id = s.stop_id)
    "#;
    let done = sqlx::query(q).bind(provider).bind(region).execute(tx.as_mut()).await?;
    tracing::info!(markers = done.rows_affected(), "[gtfs] zonal markers found");
    Ok(())
  }

  /// Checked against `stops` and `routes` in the INSERT
  async fn insert_stop_lines(
    tx: &mut Tx<'_>,
    roster: &crate::core::provider::LinesByStop,
    provider: &str,
    region: &str,
  ) -> crate::Result<()> {
    let mut stop_ids: Vec<String> = Vec::new();
    let mut lines: Vec<String> = Vec::new();

    for (stop_id, stop_lines) in roster {
      for line in stop_lines {
        stop_ids.push(stop_id.clone());
        lines.push(line.clone());
      }
    }

    let q = r#"
      INSERT INTO stop_lines (region, provider, stop_id, line)
      SELECT $3, $4, t.stop_id, t.line
      FROM UNNEST($1::text[], $2::text[]) AS t(stop_id, line)
      WHERE EXISTS (SELECT 1 FROM stops s
                    WHERE s.region = $3 AND s.provider = $4 AND s.stop_id = t.stop_id)
        AND EXISTS (SELECT 1 FROM routes r
                    WHERE r.region = $3 AND r.provider = $4 AND r.route_short_name = t.line)
      ON CONFLICT DO NOTHING
      "#;
    let done = sqlx::query(q).bind(&stop_ids).bind(&lines).bind(region).bind(provider).execute(tx.as_mut()).await?;

    tracing::info!(pairs = done.rows_affected(), received = stop_ids.len(), "[gtfs] stop-line roster");
    Ok(())
  }

  async fn insert_stops<'g>(
    tx: &mut Tx<'_>,
    gtfs: &'g Gtfs,
    provider: &str,
    region: &str,
    names: &dyn Feed,
    loaded: &mut Loaded<'g>,
  ) -> crate::Result<()> {
    let len = gtfs.stops.len();
    tracing::info!("[gtfs] syncing {} stops", len);

    let mut stop_ids: Vec<String> = Vec::with_capacity(len);
    let mut stop_names: Vec<String> = Vec::with_capacity(len);
    let mut lngs: Vec<f64> = Vec::with_capacity(len);
    let mut lats: Vec<f64> = Vec::with_capacity(len);

    for (id, stop) in &gtfs.stops {
      let (lng, lat) = match (stop.longitude, stop.latitude) {
        (Some(lng), Some(lat)) if lng != 0.0 && lat != 0.0 => (lng, lat),
        _ => {
          tracing::warn!(stop_id = %id, "[gtfs] stop has missing or zero coordinates, skipping");
          continue;
        }
      };

      loaded.stops.insert(id);
      stop_ids.push(id.clone());
      stop_names.push(stop.name.as_deref().map(|n| names.place(n)).unwrap_or_else(|| "Parada de bus".to_string()));
      lngs.push(lng);
      lats.push(lat);
    }

    let q = r#"
      INSERT INTO stops (region, provider, stop_id, stop_name, geom, area)
      SELECT $5, $6, t.stop_id, t.stop_name, p.g::geography, a.code
      FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::float8[]) AS t(stop_id, stop_name, lng, lat)
      CROSS JOIN LATERAL (SELECT ST_SetSRID(ST_MakePoint(t.lng, t.lat), 4326) AS g) p
      LEFT JOIN LATERAL (SELECT code FROM area_parts WHERE ST_Intersects(geom, p.g) LIMIT 1) a ON true
      "#;
    sqlx::query(q)
      .bind(&stop_ids)
      .bind(&stop_names)
      .bind(&lngs)
      .bind(&lats)
      .bind(region)
      .bind(provider)
      .execute(tx.as_mut())
      .await?;
    Ok(())
  }

  async fn insert_routes<'g>(
    tx: &mut Tx<'_>,
    gtfs: &'g Gtfs,
    provider: &str,
    region: &str,
    names: &dyn Feed,
    loaded: &mut Loaded<'g>,
  ) -> crate::Result<()> {
    let len = gtfs.routes.len();
    tracing::info!("[gtfs] syncing {} routes", len);

    let mut route_ids: Vec<String> = Vec::with_capacity(len);
    let mut short_names: Vec<String> = Vec::with_capacity(len);
    let mut long_names: Vec<Option<String>> = Vec::with_capacity(len);
    let mut agency_ids: Vec<Option<String>> = Vec::with_capacity(len);

    for (id, route) in &gtfs.routes {
      let short_name = match &route.short_name {
        Some(s) if !s.is_empty() => names.line(s),
        _ => {
          tracing::warn!(route_id = %id, "[gtfs] route is missing short_name, skipping");
          continue;
        }
      };

      loaded.routes.insert(id);
      route_ids.push(id.clone());
      short_names.push(short_name);
      long_names.push(route.long_name.as_deref().filter(|s| !s.is_empty()).map(|n| names.route(n)));
      agency_ids.push(route.agency_id.clone());
    }

    let q = r#"
      INSERT INTO routes (region, provider, route_id, route_short_name, route_long_name, agency_id)
      SELECT $5, $6, t.route_id, t.short_name, t.long_name, t.agency_id
      FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[]) AS t(route_id, short_name, long_name, agency_id)
      "#;
    sqlx::query(q)
      .bind(&route_ids)
      .bind(&short_names)
      .bind(&long_names)
      .bind(&agency_ids)
      .bind(region)
      .bind(provider)
      .execute(tx.as_mut())
      .await?;
    Ok(())
  }

  async fn insert_shapes<'g>(
    tx: &mut Tx<'_>,
    gtfs: &'g Gtfs,
    provider: &str,
    region: &str,
    loaded: &mut Loaded<'g>,
  ) -> crate::Result<()> {
    let len = gtfs.shapes.len();
    tracing::info!("[gtfs] syncing {} shapes", len);

    let mut shape_ids: Vec<String> = Vec::with_capacity(len);
    let mut wkts: Vec<String> = Vec::with_capacity(len);

    for (shape_id, points) in &gtfs.shapes {
      if points.is_empty() {
        tracing::warn!(shape_id = %shape_id, "[gtfs] shape has no points, skipping");
        continue;
      }

      let mut sorted = points.clone();
      sorted.sort_unstable_by_key(|p| p.sequence);

      let valid: Vec<_> = sorted.iter().filter(|p| p.longitude != 0.0 && p.latitude != 0.0).collect();

      if valid.len() < 2 {
        tracing::warn!(shape_id = %shape_id, total = points.len(), "[gtfs] shape has fewer than 2 valid points, skipping");
        continue;
      }

      let coords = valid.iter().map(|p| format!("{} {}", p.longitude, p.latitude)).collect::<Vec<_>>().join(",");
      loaded.shapes.insert(shape_id);
      shape_ids.push(shape_id.clone());
      wkts.push(format!("LINESTRING({coords})"));
    }

    let q = r#"
      INSERT INTO shapes (region, provider, shape_id, geom)
      SELECT $3, $4, t.shape_id, ST_GeomFromText(t.wkt, 4326)
      FROM UNNEST($1::text[], $2::text[]) AS t(shape_id, wkt)
      "#;
    sqlx::query(q).bind(&shape_ids).bind(&wkts).bind(region).bind(provider).execute(tx.as_mut()).await?;
    Ok(())
  }

  async fn insert_calendar<'g>(
    tx: &mut Tx<'_>,
    gtfs: &'g Gtfs,
    provider: &str,
    region: &str,
    loaded: &mut Loaded<'g>,
  ) -> crate::Result<()> {
    use chrono::NaiveDate;

    let len = gtfs.calendar.len();
    tracing::info!("[gtfs] syncing {} calendars", len);

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
        tracing::warn!(service_id = %id, start = %cal.start_date, end = %cal.end_date, "[gtfs] calendar entry has end_date before start_date, skipping");
        continue;
      }

      loaded.services.insert(id);
      service_ids.push(id.clone());
      mondays.push(cal.monday);
      tuesdays.push(cal.tuesday);
      wednesdays.push(cal.wednesday);
      thursdays.push(cal.thursday);
      fridays.push(cal.friday);
      saturdays.push(cal.saturday);
      sundays.push(cal.sunday);
      let (start, end) = (cal.start_date, cal.end_date);
      start_dates.push(naive_date(start.year(), start.month(), start.day())?);
      end_dates.push(naive_date(end.year(), end.month(), end.day())?);
    }

    let q = r#"
      INSERT INTO calendar (region, provider, service_id, monday, tuesday, wednesday, thursday, friday, saturday, sunday, start_date, end_date)
      SELECT $11, $12, t.service_id, t.mon, t.tue, t.wed, t.thu, t.fri, t.sat, t.sun, t.start_date, t.end_date
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
      .bind(region)
      .bind(provider)
      .execute(tx.as_mut())
      .await?;
    Ok(())
  }

  async fn insert_trips<'g>(
    tx: &mut Tx<'_>,
    gtfs: &'g Gtfs,
    provider: &str,
    region: &str,
    names: &dyn Feed,
    loaded: &mut Loaded<'g>,
  ) -> crate::Result<()> {
    tracing::info!("[gtfs] syncing trips");

    const CHUNK: usize = 5_000;
    let trips_vec: Vec<(&String, &gtfs_structures::Trip)> = gtfs.trips.iter().collect();
    let (mut orphans, mut unshaped) = (0usize, 0usize);

    for chunk in trips_vec.chunks(CHUNK) {
      let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut route_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut service_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut shape_ids: Vec<Option<String>> = Vec::with_capacity(chunk.len());
      let mut direction_ids: Vec<Option<i16>> = Vec::with_capacity(chunk.len());
      let mut headsigns: Vec<Option<String>> = Vec::with_capacity(chunk.len());
      let mut starts: Vec<Option<i32>> = Vec::with_capacity(chunk.len());
      let mut ends: Vec<Option<i32>> = Vec::with_capacity(chunk.len());

      for (trip_id, trip) in chunk {
        if trip.route_id.is_empty() {
          tracing::warn!(trip_id = %trip_id, "[gtfs] trip has empty route_id, skipping");
          continue;
        }
        if trip.service_id.is_empty() {
          tracing::warn!(trip_id = %trip_id, "[gtfs] trip has empty service_id, skipping");
          continue;
        }

        if !loaded.routes.contains(trip.route_id.as_str()) || !loaded.services.contains(trip.service_id.as_str()) {
          orphans += 1;
          continue;
        }
        let shape_id = trip.shape_id.clone().filter(|id| !id.is_empty());
        let shape_id = match shape_id {
          Some(id) if !loaded.shapes.contains(id.as_str()) => {
            unshaped += 1;
            None
          }
          id => id,
        };

        loaded.trips.insert(trip_id);
        trip_ids.push(trip_id.to_string());
        route_ids.push(trip.route_id.clone());
        service_ids.push(trip.service_id.clone());
        shape_ids.push(shape_id);
        direction_ids.push(trip.direction_id.as_ref().map(|d| match d {
          gtfs_structures::DirectionType::Outbound => 0,
          gtfs_structures::DirectionType::Inbound => 1,
        }));
        headsigns.push(trip.trip_headsign.as_deref().map(|n| names.place(n)));
        let times = trip.stop_times.iter();
        starts.push(times.clone().filter_map(|st| st.departure_time.or(st.arrival_time)).min().map(|t| t as i32));
        ends.push(times.filter_map(|st| st.arrival_time.or(st.departure_time)).max().map(|t| t as i32));
      }

      let q = r#"
        INSERT INTO trips(region, provider, trip_id, route_id, service_id, shape_id, direction_id, trip_headsign, starts, ends)
        SELECT $7, $8, t.trip_id, t.route_id, t.service_id, t.shape_id, t.direction_id, t.trip_headsign,
          CASE WHEN t.starts IS NOT NULL THEN make_interval(secs => t.starts::float8) END,
          CASE WHEN t.ends IS NOT NULL THEN make_interval(secs => t.ends::float8) END
        FROM UNNEST($1::text[], $2::text[], $3::text[], $4::text[], $5::int2[], $6::text[], $9::int4[], $10::int4[])
          AS t(trip_id, route_id, service_id, shape_id, direction_id, trip_headsign, starts, ends)
        ON CONFLICT (region, provider, trip_id) DO NOTHING
        "#;
      sqlx::query(q)
        .bind(&trip_ids)
        .bind(&route_ids)
        .bind(&service_ids)
        .bind(&shape_ids)
        .bind(&direction_ids)
        .bind(&headsigns)
        .bind(region)
        .bind(provider)
        .bind(&starts)
        .bind(&ends)
        .execute(tx.as_mut())
        .await?;
    }

    if orphans > 0 || unshaped > 0 {
      tracing::warn!(orphans, unshaped, "[gtfs] trips referencing unloaded routes, services or shapes");
    }
    Ok(())
  }

  async fn insert_stop_times(
    tx: &mut Tx<'_>,
    gtfs: &Gtfs,
    provider: &str,
    region: &str,
    loaded: &Loaded<'_>,
  ) -> crate::Result<()> {
    tracing::info!("[gtfs] syncing stop times");

    const CHUNK: usize = 5_000;

    let all_stop_times: Vec<(&str, &gtfs_structures::StopTime)> = gtfs
      .trips
      .iter()
      .flat_map(|(trip_id, trip)| trip.stop_times.iter().map(move |st| (trip_id.as_str(), st)))
      .collect();
    let mut orphans = 0usize;

    for chunk in all_stop_times.chunks(CHUNK) {
      let mut trip_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut stop_ids: Vec<String> = Vec::with_capacity(chunk.len());
      let mut sequences: Vec<i32> = Vec::with_capacity(chunk.len());
      let mut arrivals: Vec<Option<i32>> = Vec::with_capacity(chunk.len());
      let mut departures: Vec<Option<i32>> = Vec::with_capacity(chunk.len());

      for (trip_id, st) in chunk {
        if !loaded.trips.contains(trip_id) || !loaded.stops.contains(st.stop.id.as_str()) {
          orphans += 1;
          continue;
        }
        if st.arrival_time.is_none() && st.departure_time.is_none() {
          // tracing::warn!(trip_id = %trip_id, stop_id = %st.stop.id, sequence = st.stop_sequence, "[gtfs] stop time has no arrival or departure, skipping");
          continue;
        }

        trip_ids.push(trip_id.to_string());
        stop_ids.push(st.stop.id.clone());
        sequences.push(st.stop_sequence as i32);
        arrivals.push(st.arrival_time.map(|t| t as i32));
        departures.push(st.departure_time.map(|t| t as i32));
      }

      let q = r#"
        INSERT INTO stop_times(region, provider, trip_id, stop_id, stop_sequence, arrival_time, departure_time)
        SELECT $6, $7, t.trip_id, t.stop_id, t.stop_sequence,
          CASE WHEN t.arr IS NOT NULL THEN make_interval(secs => t.arr::float8) END,
          CASE WHEN t.dep IS NOT NULL THEN make_interval(secs => t.dep::float8) END
        FROM UNNEST($1::text[], $2::text[], $3::int4[], $4::int4[], $5::int4[])
          AS t(trip_id, stop_id, stop_sequence, arr, dep)
        ON CONFLICT (region, provider, trip_id, stop_sequence) DO NOTHING
        "#;
      sqlx::query(q)
        .bind(&trip_ids)
        .bind(&stop_ids)
        .bind(&sequences)
        .bind(&arrivals)
        .bind(&departures)
        .bind(region)
        .bind(provider)
        .execute(tx.as_mut())
        .await?;
    }

    if orphans > 0 {
      tracing::warn!(orphans, "[gtfs] stop times referencing unloaded trips or stops");
    }
    Ok(())
  }

  async fn end(mut tx: Tx<'_>, gtfs: &Gtfs, version: &str, provider: &str) -> crate::Result<()> {
    let q = r#"
    INSERT INTO gtfs_metadata (provider, version, synced_at) VALUES ($2, $1, NOW())
    ON CONFLICT (provider) DO UPDATE SET version = EXCLUDED.version, synced_at = EXCLUDED.synced_at
    "#;
    sqlx::query(q).bind(version).bind(provider).execute(tx.as_mut()).await?;

    tx.commit().await?;

    let n_stops = gtfs.stops.len();
    let n_routes = gtfs.routes.len();
    let n_shapes = gtfs.shapes.len();
    let n_trips = gtfs.trips.len();
    let n_stop_times: usize = gtfs.trips.values().map(|t| t.stop_times.len()).sum();
    tracing::info!(v = version, n_stops, n_routes, n_shapes, n_trips, n_stop_times, "[gtfs] sync complete");

    Ok(())
  }
}

// components, to avoid depending on jiff
fn naive_date(year: i16, month: i8, day: i8) -> crate::Result<chrono::NaiveDate> {
  chrono::NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
    .ok_or_else(|| crate::Error::External(format!("invalid date in calendar.txt: {year}-{month}-{day}")))
}
