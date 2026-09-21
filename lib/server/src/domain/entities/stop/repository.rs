use super::super::bus::Bus;
use super::{
  ApproachingBusRow, PastStopBusRow, ScheduledLine, Stop, StopArrival, StopDetail, StopLine, StopLines, StopRoute,
  StopSchedule, StopSummary,
};

use std::collections::HashMap;

impl StopLine {
  pub async fn get(db: &crate::Db, stop_id: &str) -> crate::Result<StopLines> {
    use chrono::{Local, Timelike};

    let now = Local::now();
    let today_secs = now.num_seconds_from_midnight() as i64;

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct Row {
      line: String,
      destination: Option<String>,
      secs: Option<i64>,
      rn: i64,
    }

    let q = r#"
    WITH all_departures AS (
        -- Today's normal trips (departure_time up to 24:00:00)
        SELECT
            r.route_short_name                                             AS line,
            t.trip_headsign                                                AS destination,
            EXTRACT(EPOCH FROM (st.departure_time - $1::interval))::bigint AS secs
        FROM stop_times  st
        JOIN trips       t  ON t.trip_id    = st.trip_id
        JOIN routes      r  ON r.route_id   = t.route_id
        JOIN calendar    c  ON c.service_id = t.service_id
        WHERE st.stop_id = $2
          AND st.departure_time > $1::interval
          AND st.departure_time <= INTERVAL '1 day'
          AND c.start_date <= $3::date
          AND c.end_date   >= $3::date
          AND CASE EXTRACT(DOW FROM $3::date)::int
                WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
                WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
                WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
                WHEN 0 THEN c.sunday
              END = true

        UNION ALL

        -- Yesterday's overnight trips (departure_time > 24:00:00)
        SELECT
            r.route_short_name                                                             AS line,
            t.trip_headsign                                                                AS destination,
            EXTRACT(EPOCH FROM (st.departure_time - INTERVAL '1 day' - $1::interval))::bigint AS secs
        FROM stop_times  st
        JOIN trips       t  ON t.trip_id    = st.trip_id
        JOIN routes      r  ON r.route_id   = t.route_id
        JOIN calendar    c  ON c.service_id = t.service_id
        WHERE st.stop_id = $2
          AND st.departure_time > INTERVAL '1 day'
          AND st.departure_time - INTERVAL '1 day' > $1::interval
          AND c.start_date <= $3::date - 1
          AND c.end_date   >= $3::date - 1
          AND CASE EXTRACT(DOW FROM $3::date - 1)::int
                WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
                WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
                WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
                WHEN 0 THEN c.sunday
              END = true
    ),
    ranked AS (
        SELECT
            line, destination, secs,
            ROW_NUMBER() OVER (
                PARTITION BY line
                ORDER BY secs
            ) AS rn
        FROM all_departures
    )
    SELECT line, destination, secs, rn
    FROM ranked
    WHERE rn <= 3
    ORDER BY line, rn
    "#;
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(format!("{} seconds", today_secs))
      .bind(stop_id)
      .bind(now.date_naive())
      .fetch_all(db)
      .await?;

    let mut map = StopLines::new();
    for row in rows {
      let eta = row.secs.map(|s| chrono::Duration::seconds(s).num_minutes());
      let entry = map.entry(row.line.clone()).or_insert_with(|| StopLine {
        line: row.line,
        destination: row.destination,
        company: String::new(),
        arrivals: vec![],
      });
      entry.arrivals.push(StopArrival { eta, live: false, bus: None });
    }

    let mut lines: Vec<StopLine> = map.into_values().collect();
    lines.sort_by(|a, b| a.cmp_by_first_eta(b));
    lines.truncate(20);
    Ok(lines.into_iter().map(|l| (l.line.clone(), l)).collect())
  }
}

//- sql
impl StopSchedule {
  pub async fn get(db: &crate::Db, stop_id: &str) -> crate::Result<Self> {
    use chrono::Local;

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      destination: Option<String>,
      secs: i64,
    }

    // yesterday past 24 h
    let q = r#"
    SELECT r.route_short_name AS line,
           t.trip_headsign     AS destination,
           EXTRACT(EPOCH FROM st.departure_time)::bigint AS secs
    FROM stop_times st
    JOIN trips    t ON t.trip_id    = st.trip_id
    JOIN routes   r ON r.route_id   = t.route_id
    JOIN calendar c ON c.service_id = t.service_id
    WHERE st.stop_id = $1
      AND st.departure_time <= INTERVAL '1 day'
      AND c.start_date <= $2::date
      AND c.end_date   >= $2::date
      AND CASE EXTRACT(DOW FROM $2::date)::int
            WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
            WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
            WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
            WHEN 0 THEN c.sunday
          END = true

    UNION ALL

    SELECT r.route_short_name AS line,
           t.trip_headsign     AS destination,
           EXTRACT(EPOCH FROM st.departure_time - INTERVAL '1 day')::bigint AS secs
    FROM stop_times st
    JOIN trips    t ON t.trip_id    = st.trip_id
    JOIN routes   r ON r.route_id   = t.route_id
    JOIN calendar c ON c.service_id = t.service_id
    WHERE st.stop_id = $1
      AND st.departure_time > INTERVAL '1 day'
      AND c.start_date <= $2::date - 1
      AND c.end_date   >= $2::date - 1
      AND CASE EXTRACT(DOW FROM $2::date - 1)::int
            WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
            WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
            WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
            WHEN 0 THEN c.sunday
          END = true
    "#;
    let now = Local::now();
    let rows: Vec<Row> = sqlx::query_as(q).bind(stop_id).bind(now.date_naive()).fetch_all(db).await?;

    let mut by_line: HashMap<String, (Vec<i64>, HashMap<String, usize>)> = HashMap::new();
    for row in rows {
      let entry = by_line.entry(row.line).or_default();
      entry.0.push(row.secs);
      if let Some(destination) = row.destination {
        *entry.1.entry(destination).or_default() += 1;
      }
    }

    let mut lines: Vec<ScheduledLine> = by_line
      .into_iter()
      .map(|(line, (mut departures, destinations))| {
        departures.sort_unstable();
        let destination = destinations.into_iter().max_by_key(|(_, n)| *n).map(|(d, _)| d);
        ScheduledLine { line, destination, departures }
      })
      .collect();

    lines.sort_by(|a, b| crate::helpers::cmp_line(&a.line, &b.line));

    Ok(Self { service_date: now.format("%Y-%m-%d").to_string(), lines })
  }
}

//- sql
impl Stop {
  pub async fn get(db: &crate::Db, stop_id: &str) -> crate::Result<Self> {
    #[derive(sqlx::FromRow)]
    struct StopRow {
      stop_id: String,
      stop_name: String,
      lat: f64,
      lng: f64,
      bearing_rad: Option<f64>,
    }

    let q = r#"
    SELECT s.stop_id, s.stop_name, s.bearing_rad,
           ST_Y(s.geom::geometry) AS lat,
           ST_X(s.geom::geometry) AS lng
    FROM stops s WHERE s.stop_id = $1
    "#;
    let row: Option<StopRow> = sqlx::query_as(q).bind(stop_id).fetch_optional(db).await?;
    let row = row.ok_or_else(|| crate::Error::NotFound(format!("stop {stop_id} not found")))?;

    #[derive(sqlx::FromRow)]
    struct RouteRow {
      line: String,
      destination: Option<String>,
    }

    // natural line order, destinations by frequency
    let q = r#"
    SELECT r.route_short_name AS line, t.trip_headsign AS destination
    FROM stop_times st
    JOIN trips  t ON t.trip_id  = st.trip_id
    JOIN routes r ON r.route_id = t.route_id
    WHERE st.stop_id = $1
    GROUP BY r.route_short_name, t.trip_headsign
    ORDER BY NULLIF(regexp_replace(r.route_short_name, '\D', '', 'g'), '')::bigint NULLS LAST,
             r.route_short_name,
             COUNT(*) DESC
    "#;
    let rows: Vec<RouteRow> = sqlx::query_as(q).bind(stop_id).fetch_all(db).await?;

    let mut lines: Vec<StopRoute> = Vec::new();
    for r in rows {
      let dest = r.destination;
      match lines.last_mut() {
        Some(last) if last.line == r.line => {
          if let Some(d) = dest
            && !last.destinations.contains(&d)
          {
            last.destinations.push(d);
          }
        }
        _ => lines.push(StopRoute { line: r.line, destinations: dest.into_iter().collect() }),
      }
    }

    Ok(Self {
      stop_id: row.stop_id,
      stop_name: row.stop_name,
      lat: row.lat,
      lng: row.lng,
      bearing_rad: row.bearing_rad,
      lines,
    })
  }

  pub async fn list(db: &crate::Db) -> crate::Result<Vec<StopSummary>> {
    let q = r#"
    SELECT s.stop_id, s.stop_name,
           ST_Y(s.geom::geometry) AS lat,
           ST_X(s.geom::geometry) AS lng
    FROM stops s ORDER BY s.stop_id
    "#;
    Ok(sqlx::query_as(q).fetch_all(db).await?)
  }
}

//- cache
impl Stop {
  fn stop_key(stop_id: &str) -> String {
    format!("stop:{stop_id}")
  }

  pub async fn cache_get(cache: &crate::Cache, stop_id: &str) -> Option<Self> {
    cache.get(&Self::stop_key(stop_id)).await
  }

  pub async fn cache_set(cache: &crate::Cache, stop_id: &str, stop: &Self) {
    cache.set(&Self::stop_key(stop_id), stop, cache.until_sync()).await;
  }

  pub async fn cache_get_list(cache: &crate::Cache) -> Option<Vec<StopSummary>> {
    cache.get("stops:all").await
  }

  pub async fn cache_set_list(cache: &crate::Cache, stops: &[StopSummary]) {
    cache.set("stops:all", &stops, cache.until_sync()).await;
  }
}

// cache
use crate::cache::CacheRw;

impl StopSchedule {
  fn schedule_key(stop_id: &str, date: &str) -> String {
    format!("schedule:{stop_id}:{date}")
  }

  pub async fn cache_get(cache: &crate::Cache, stop_id: &str, date: &str) -> Option<Self> {
    cache.get(&Self::schedule_key(stop_id, date)).await
  }

  /// Expires at midnight
  pub async fn cache_set(cache: &crate::Cache, stop_id: &str, schedule: &Self) {
    let ttl = crate::helpers::secs_to_midnight().min(cache.until_sync());
    cache.set(&Self::schedule_key(stop_id, &schedule.service_date), schedule, ttl).await;
  }
}
impl StopDetail {
  //# GET /stops/:stop_id
  const TTL: u64 = 10;

  fn stop_detail_key(stop_id: &str) -> String {
    format!("stop_detail:{stop_id}")
  }

  pub async fn cache_get(cache: &crate::Cache, stop_id: &str) -> Option<StopDetail> {
    cache.get(&Self::stop_detail_key(stop_id)).await
  }
  pub async fn cache_set(cache: &crate::Cache, stop_id: &str, detail: &StopDetail) {
    cache.set(&Self::stop_detail_key(stop_id), detail, Self::TTL).await;
  }
}

// bus-stop projection

impl StopArrival {
  /// Remaining distance of buses before the stop
  pub async fn query_approaching(
    db: &crate::Db,
    stop_id: &str,
    buses: &[Bus],
  ) -> crate::Result<Vec<ApproachingBusRow>> {
    if buses.is_empty() {
      return Ok(Vec::new());
    }

    let mut p_lines: Vec<String> = Vec::with_capacity(buses.len());
    let mut p_lats: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_lngs: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_dests: Vec<Option<String>> = Vec::with_capacity(buses.len());
    let mut p_ids: Vec<Option<String>> = Vec::with_capacity(buses.len());

    for bus in buses {
      p_lines.push(bus.line.clone());
      p_lats.push(bus.location.lat);
      p_lngs.push(bus.location.lng);
      p_dests.push(bus.destination.clone());
      p_ids.push(Some(bus.id.clone()));
    }

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      bus_id: Option<String>,
      remaining_m: Option<f64>,
    }

    let q = r#"
      WITH line_shapes AS (
          SELECT DISTINCT ON (t.shape_id)
              r.route_short_name  AS line,
              t.trip_headsign     AS headsign,
              sh.geom             AS shape,
              ST_LineLocatePoint(sh.geom, s.geom::geometry) AS stop_frac
          FROM routes     r
          JOIN trips      t  ON t.route_id  = r.route_id
          JOIN shapes     sh ON sh.shape_id = t.shape_id
          JOIN stop_times st ON st.trip_id  = t.trip_id
          JOIN stops      s  ON s.stop_id   = st.stop_id
          WHERE s.stop_id = $5
            AND r.route_short_name = ANY($1::text[])
          ORDER BY t.shape_id
      ),
      buses AS (
          SELECT *
          FROM unnest($1::text[], $2::float8[], $3::float8[], $4::text[], $6::text[])
            AS t(bus_line, bus_lat, bus_lng, bus_dest, bus_id)
      )
      SELECT DISTINCT ON (b.bus_id)
          b.bus_line AS line, b.bus_id,
          ST_Length(ST_LineSubstring(
              ls.shape,
              ST_LineLocatePoint(ls.shape,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)),
              ls.stop_frac
          )::geography) AS remaining_m
      FROM buses b
      JOIN line_shapes ls ON ls.line = b.bus_line
        AND (b.bus_dest IS NULL
             OR ls.headsign IS NULL
             OR UPPER(TRIM(ls.headsign)) = UPPER(TRIM(b.bus_dest)))
      WHERE ST_LineLocatePoint(ls.shape,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) < ls.stop_frac
        AND ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography) < 500
      ORDER BY b.bus_id,
          ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography)
      "#;
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(&p_lines)
      .bind(&p_lats)
      .bind(&p_lngs)
      .bind(&p_dests)
      .bind(stop_id)
      .bind(&p_ids)
      .fetch_all(db)
      .await?;

    Ok(
      rows
        .into_iter()
        .filter_map(|r| {
          let remaining_m = r.remaining_m.filter(|&m| m > 0.0)?;
          Some(ApproachingBusRow { line: r.line, bus_id: r.bus_id.unwrap_or_default(), remaining_m })
        })
        .collect(),
    )
  }

  /// Distance (m) of buses past the stop, within a margin
  pub async fn query_past_stop(db: &crate::Db, stop_id: &str, buses: &[Bus]) -> crate::Result<Vec<PastStopBusRow>> {
    if buses.is_empty() {
      return Ok(Vec::new());
    }

    let mut p_lines: Vec<String> = Vec::with_capacity(buses.len());
    let mut p_lats: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_lngs: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_dests: Vec<Option<String>> = Vec::with_capacity(buses.len());
    let mut p_ids: Vec<Option<String>> = Vec::with_capacity(buses.len());

    for bus in buses {
      p_lines.push(bus.line.clone());
      p_lats.push(bus.location.lat);
      p_lngs.push(bus.location.lng);
      p_dests.push(bus.destination.clone());
      p_ids.push(Some(bus.id.clone()));
    }

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      bus_id: Option<String>,
      past_m: Option<f64>,
    }

    let q = r#"
      WITH line_shapes AS (
          SELECT DISTINCT ON (t.shape_id)
              r.route_short_name  AS line,
              t.trip_headsign     AS headsign,
              sh.geom             AS shape,
              ST_LineLocatePoint(sh.geom, s.geom::geometry) AS stop_frac
          FROM routes     r
          JOIN trips      t  ON t.route_id  = r.route_id
          JOIN shapes     sh ON sh.shape_id = t.shape_id
          JOIN stop_times st ON st.trip_id  = t.trip_id
          JOIN stops      s  ON s.stop_id   = st.stop_id
          WHERE s.stop_id = $4
            AND r.route_short_name = ANY($1::text[])
          ORDER BY t.shape_id
      ),
      buses AS (
          SELECT *
          FROM unnest($1::text[], $2::float8[], $3::float8[], $5::text[], $6::text[])
            AS t(bus_line, bus_lat, bus_lng, bus_dest, bus_id)
      )
      SELECT DISTINCT ON (b.bus_id)
          b.bus_line AS line, b.bus_id,
          ST_Length(ST_LineSubstring(
              ls.shape,
              ls.stop_frac,
              ST_LineLocatePoint(ls.shape,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326))
          )::geography) AS past_m
      FROM buses b
      JOIN line_shapes ls ON ls.line = b.bus_line
        AND (b.bus_dest IS NULL
             OR ls.headsign IS NULL
             OR UPPER(TRIM(ls.headsign)) = UPPER(TRIM(b.bus_dest)))
      WHERE ST_LineLocatePoint(ls.shape,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) >= ls.stop_frac
        AND ST_LineLocatePoint(ls.shape,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) <= ls.stop_frac + 0.05
        AND ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography) < 500
      ORDER BY b.bus_id,
          ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography)
      "#;
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(&p_lines)
      .bind(&p_lats)
      .bind(&p_lngs)
      .bind(stop_id)
      .bind(&p_dests)
      .bind(&p_ids)
      .fetch_all(db)
      .await?;

    Ok(
      rows
        .into_iter()
        .map(|r| PastStopBusRow { line: r.line, bus_id: r.bus_id.unwrap_or_default(), past_m: r.past_m.unwrap_or(0.0) })
        .collect(),
    )
  }
}
