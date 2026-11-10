use super::{
  DayType, DirectionInfo, Route, RouteSchedule, RouteShape, RouteStop, RouteTimes, ScheduleDeparture, ScheduleHour,
};
use crate::scope::Scope;

use chrono::{Datelike, NaiveDate, Timelike, Weekday};
use std::collections::{HashMap, VecDeque};

//- sql
impl Route {
  pub async fn search(db: &crate::Db, q: &str) -> crate::Result<Vec<Self>> {
    let pattern = format!("%{}%", q);

    let routes = sqlx::query_as::<_, Route>(
      r#"
    SELECT provider || '-' || route_id AS route_id, route_short_name, route_long_name, agency_id, region, area
    FROM routes
    WHERE route_short_name ILIKE $1 OR route_long_name ILIKE $1
    ORDER BY
      CASE WHEN UPPER(route_short_name) = UPPER($2) THEN 0 ELSE 1 END,
      route_short_name
    LIMIT 20
    "#,
    )
    .bind(&pattern)
    .bind(q)
    .fetch_all(db)
    .await?;

    Ok(routes)
  }
}

impl Route {
  pub async fn list(db: &crate::Db) -> crate::Result<Vec<Self>> {
    let q = r#"
    SELECT provider || '-' || route_id AS route_id, route_short_name, route_long_name, agency_id, region, area
    FROM routes
    ORDER BY NULLIF(regexp_replace(route_short_name, '\D', '', 'g'), '')::bigint NULLS LAST,
             route_short_name, route_id
    "#;
    Ok(sqlx::query_as::<_, Route>(q).fetch_all(db).await?)
  }

  pub async fn load_list(state: &crate::Transit) -> crate::Result<Vec<Self>> {
    let cache = &state.cache;
    cache
      .get_or_load(
        &crate::cache::Cache::key(crate::cache::Cache::SHARED, "routes:all"),
        cache.until_sync(),
        Self::list(&state.db),
      )
      .await
  }
}

struct Resolved {
  route_id: String,
  trip_id: String,
  shape_id: Option<String>,
  direction_id: i16,
  headsign: Option<String>,
  available_directions: Vec<DirectionInfo>,
}

impl RouteShape {
  pub async fn get(db: &crate::Db, scope: &Scope, route_id: &str, direction: i16) -> crate::Result<Self> {
    let directions = Self::directions(db, scope, route_id).await?;
    let Some(first) = directions.first() else {
      return Err(crate::Error::NotFound(format!("no trips found for route {route_id}")));
    };

    // direction_id is a preference
    let resolved = directions.iter().find(|r| r.direction_id == direction).unwrap_or(first);
    let (direction, headsign) = (resolved.direction_id, resolved.headsign.clone());

    // most frequent shape of the destination, else the trip with most stops
    let q = r#"
      SELECT t.trip_id, t.shape_id FROM trips t
      WHERE t.region = $3 AND t.provider = $4 AND t.route_id = $1 AND t.direction_id = $2
      ORDER BY t.shape_id IS NULL,
        t.trip_headsign IS NOT DISTINCT FROM $5 DESC,
        count(*) OVER (PARTITION BY t.shape_id, t.trip_headsign) DESC,
        CASE WHEN t.shape_id IS NULL THEN -(SELECT count(*) FROM stop_times st
          WHERE st.region = t.region AND st.provider = t.provider AND st.trip_id = t.trip_id) END,
        t.trip_id
      LIMIT 1
    "#;
    let (trip_id, shape_id): (String, Option<String>) = sqlx::query_as(q)
      .bind(route_id)
      .bind(direction)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(&headsign)
      .fetch_optional(db)
      .await?
      .ok_or_else(|| crate::Error::NotFound(format!("no trips found for route {route_id} direction {direction}")))?;

    let resolved = Resolved {
      route_id: route_id.to_string(),
      trip_id,
      shape_id,
      direction_id: direction,
      headsign,
      available_directions: directions,
    };
    Self::build(db, scope, resolved).await
  }

  /// Route of a live bus, from its shape
  pub async fn get_by_shape(db: &crate::Db, scope: &Scope, shape_id: &str) -> crate::Result<Self> {
    #[derive(sqlx::FromRow)]
    struct Row {
      route_id: String,
      trip_id: String,
      direction_id: i16,
      trip_headsign: Option<String>,
    }

    let q = r#"
      SELECT route_id, MIN(trip_id) AS trip_id, COALESCE(direction_id, 0)::int2 AS direction_id, trip_headsign
      FROM trips
      WHERE region = $2 AND provider = $3 AND shape_id = $1
      GROUP BY route_id, direction_id, trip_headsign
      ORDER BY COUNT(*) DESC
      LIMIT 1
    "#;
    let row = sqlx::query_as::<_, Row>(q)
      .bind(shape_id)
      .bind(scope.region.id())
      .bind(scope.provider)
      .fetch_optional(db)
      .await?
      .ok_or_else(|| crate::Error::NotFound(format!("no trips found for shape {shape_id}")))?;

    let available_directions = Self::directions(db, scope, &row.route_id).await?;
    let resolved = Resolved {
      route_id: row.route_id,
      trip_id: row.trip_id,
      shape_id: Some(shape_id.to_string()),
      direction_id: row.direction_id,
      headsign: row.trip_headsign,
      available_directions,
    };
    Self::build(db, scope, resolved).await
  }

  // most frequent headsign per direction
  async fn directions(db: &crate::Db, scope: &Scope, route_id: &str) -> crate::Result<Vec<DirectionInfo>> {
    #[derive(sqlx::FromRow)]
    struct Row {
      direction_id: i16,
      trip_headsign: Option<String>,
    }

    let q = r#"
      SELECT DISTINCT ON (direction_id) direction_id, trip_headsign
      FROM (
        SELECT direction_id, trip_headsign, COUNT(*) AS trips
        FROM trips
        WHERE region = $2 AND provider = $3 AND route_id = $1 AND direction_id IS NOT NULL
        GROUP BY direction_id, trip_headsign
      ) t
      ORDER BY direction_id, trips DESC
    "#;
    let rows =
      sqlx::query_as::<_, Row>(q).bind(route_id).bind(scope.region.id()).bind(scope.provider).fetch_all(db).await?;
    Ok(rows.into_iter().map(|r| DirectionInfo { direction_id: r.direction_id, headsign: r.trip_headsign }).collect())
  }

  async fn build(db: &crate::Db, scope: &Scope, r: Resolved) -> crate::Result<Self> {
    let shape = match &r.shape_id {
      Some(shape_id) => {
        let q = r#"
          SELECT ST_AsGeoJSON(geom, 5) FROM shapes
          WHERE region = $2 AND provider = $3 AND shape_id = $1
        "#;
        let geojson_str: Option<String> =
          sqlx::query_scalar(q).bind(shape_id).bind(scope.region.id()).bind(scope.provider).fetch_optional(db).await?;
        let shape_geojson = geojson_str.ok_or_else(|| crate::Error::NotFound(format!("no shape {shape_id}")))?;
        serde_json::from_str(&shape_geojson).unwrap_or(serde_json::Value::Null)
      }
      None => serde_json::Value::Null,
    };

    let q = r#"
      SELECT s.provider || '-' || s.stop_id AS stop_id,
             COALESCE(s.cluster, s.provider || '-' || s.stop_id) AS cluster,
             s.stop_name, s.area, st.stop_sequence, round(ST_Y(s.geom::geometry)::numeric, 5)::float8 AS lat, round(ST_X(s.geom::geometry)::numeric, 5)::float8 AS lng, s.bearing_rad,
             s.station, s.platform
      FROM stop_times st JOIN stops s ON s.region = st.region AND s.provider = st.provider AND s.stop_id = st.stop_id
      WHERE st.region = $2 AND st.provider = $3 AND st.trip_id = $1
      ORDER BY st.stop_sequence
    "#;
    let stops = sqlx::query_as::<_, RouteStop>(q)
      .bind(&r.trip_id)
      .bind(scope.region.id())
      .bind(scope.provider)
      .fetch_all(db)
      .await?;

    Ok(RouteShape {
      route_id: crate::scope::Scope::qualify(scope.provider, &r.route_id),
      shape_id: r.shape_id,
      direction_id: r.direction_id,
      headsign: r.headsign,
      shape,
      stops,
      available_directions: r.available_directions,
    })
  }
}

impl RouteTimes {
  /// Next trip times through `stop`, aligned with `stops`
  async fn get(db: &crate::Db, scope: &Scope, shape: &RouteShape, stop: Option<&str>) -> crate::Result<Self> {
    let prefix = format!("{}-", scope.provider);
    let route_id = shape.route_id.strip_prefix(&prefix).unwrap_or(&shape.route_id);

    // yesterday past 24 h, 1 min grace, local time from here
    let now = chrono::Local::now();
    let q = r#"
      WITH day_trips AS (
        SELECT t.trip_id, t.starts, d.shift
        FROM trips t
        JOIN calendar c ON c.region = t.region AND c.provider = t.provider AND c.service_id = t.service_id
        JOIN (VALUES ($8::date, 0), ($8::date - 1, 86400)) AS d(day, shift)
          ON c.start_date <= d.day AND c.end_date >= d.day
         AND CASE EXTRACT(DOW FROM d.day)::int
               WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
               WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
               WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
               WHEN 0 THEN c.sunday
             END = true
        WHERE t.region = $1 AND t.provider = $2 AND t.route_id = $3
          AND COALESCE(t.direction_id, 0) = $4
          AND t.shape_id IS NOT DISTINCT FROM $5::text
          AND ($5::text IS NOT NULL OR t.trip_headsign IS NOT DISTINCT FROM $6::text)
      ),
      anchor_stop AS (
        SELECT v.trip_id, v.shift, EXTRACT(EPOCH FROM v.starts)::int - v.shift AS secs
        FROM day_trips v WHERE $7::text IS NULL AND v.starts IS NOT NULL
        UNION ALL
        SELECT v.trip_id, v.shift, EXTRACT(EPOCH FROM st.departure_time)::int - v.shift
        FROM day_trips v
        JOIN stop_times st ON st.region = $1 AND st.provider = $2 AND st.trip_id = v.trip_id AND st.stop_id = $7
      ),
      next_trip AS (
        SELECT trip_id, shift FROM anchor_stop
        WHERE secs >= $9 - 60
        ORDER BY secs LIMIT 1
      )
      SELECT st.stop_id, EXTRACT(EPOCH FROM st.arrival_time)::int - p.shift AS secs
      FROM next_trip p
      JOIN stop_times st ON st.region = $1 AND st.provider = $2 AND st.trip_id = p.trip_id
      ORDER BY st.stop_sequence
    "#;
    let rows: Vec<(String, i32)> = sqlx::query_as(q)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(route_id)
      .bind(shape.direction_id)
      .bind(&shape.shape_id)
      .bind(&shape.headsign)
      .bind(stop)
      .bind(now.date_naive())
      .bind(now.num_seconds_from_midnight() as i32)
      .fetch_all(db)
      .await?;
    // by appearance, loops repeat stops
    let mut by_stop: HashMap<String, VecDeque<i32>> = HashMap::new();
    for (stop_id, secs) in rows {
      by_stop.entry(stop_id).or_default().push_back(secs);
    }
    let times = if by_stop.is_empty() {
      vec![]
    } else {
      shape.stops.iter().map(|s| by_stop.get_mut(s.stop_id.strip_prefix(&prefix)?)?.pop_front()).collect()
    };
    Ok(Self { times })
  }
}

impl DayType {
  pub fn of(date: NaiveDate) -> Self {
    match date.weekday() {
      Weekday::Sat => Self::Saturday,
      Weekday::Sun => Self::Sunday,
      _ => Self::Workday,
    }
  }

  /// Today if it matches, else the next one
  pub fn next_date(self, today: NaiveDate) -> NaiveDate {
    today.iter_days().find(|d| Self::of(*d) == self).unwrap_or(today)
  }
}

impl RouteSchedule {
  /// Departures at the stop from any shape, last stops excluded
  async fn get(
    db: &crate::Db,
    scope: &Scope,
    route_id: &str,
    direction: i16,
    stop: &str,
    day: DayType,
    date: NaiveDate,
  ) -> crate::Result<Self> {
    let q = r#"
      SELECT DISTINCT ON (secs) secs, (secs / 60 % 60) AS minute, zonal, interpolated
      FROM (
        SELECT EXTRACT(EPOCH FROM st.departure_time)::int AS secs, st.zonal, st.interpolated
        FROM trips t
        JOIN calendar c ON c.region = t.region AND c.provider = t.provider AND c.service_id = t.service_id
        JOIN stop_times st ON st.region = t.region AND st.provider = t.provider AND st.trip_id = t.trip_id
        WHERE t.region = $1 AND t.provider = $2 AND t.route_id = $3
          AND COALESCE(t.direction_id, 0) = $4
          AND st.stop_id = $5
          AND c.start_date <= $6 AND c.end_date >= $6
          AND CASE EXTRACT(DOW FROM $6::date)::int
                WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
                WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
                WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
                WHEN 0 THEN c.sunday
              END = true
          AND EXISTS (SELECT 1 FROM stop_times nx WHERE nx.region = st.region AND nx.provider = st.provider
                      AND nx.trip_id = st.trip_id AND nx.stop_sequence > st.stop_sequence)
      ) d
      ORDER BY secs
    "#;
    let departures: Vec<ScheduleDeparture> = sqlx::query_as(q)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(route_id)
      .bind(direction)
      .bind(stop)
      .bind(date)
      .fetch_all(db)
      .await?;

    let mut hours: Vec<ScheduleHour> = Vec::new();
    for departure in departures {
      let (hour, late) = ((departure.secs / 3600 % 24) as u32, departure.secs >= 86_400);
      match hours.last_mut() {
        Some(h) if h.hour == hour && h.late == late => h.departures.push(departure),
        _ => hours.push(ScheduleHour { hour, late, departures: vec![departure] }),
      }
    }
    Ok(Self { day, date: date.format("%Y-%m-%d").to_string(), hours })
  }
}

//- cache
impl Route {
  fn search_key(q: &str) -> String {
    crate::cache::Cache::key(crate::cache::Cache::SHARED, &format!("routes:search:{}", q.to_lowercase()))
  }

  pub async fn load_search(state: &crate::Transit, q: &str) -> crate::Result<Vec<Self>> {
    let cache = &state.cache;
    cache.get_or_load(&Self::search_key(q), cache.until_sync(), Self::search(&state.db, q)).await
  }
}

impl RouteShape {
  fn shape_key(scope: &Scope, line: &str, direction: i16) -> String {
    crate::cache::Cache::key(scope.region.id(), &format!("routes:shape:{}:{line}:{direction}", scope.provider))
  }

  pub async fn load(state: &crate::Transit, scope: &Scope, route_id: &str, direction: i16) -> crate::Result<Self> {
    let cache = &state.cache;
    let key = Self::shape_key(scope, route_id, direction);
    cache.get_or_load(&key, cache.until_sync(), Self::get(&state.db, scope, route_id, direction)).await
  }

  pub async fn load_by_shape(state: &crate::Transit, scope: &Scope, shape_id: &str) -> crate::Result<Self> {
    let cache = &state.cache;
    let key = crate::cache::Cache::key(scope.region.id(), &format!("routes:shape_id:{}:{shape_id}", scope.provider));
    cache.get_or_load(&key, cache.until_sync(), Self::get_by_shape(&state.db, scope, shape_id)).await
  }
}

impl RouteTimes {
  /// Short TTL
  const TTL: u64 = 30;

  /// On the shown shape, or the direction's main one
  pub async fn load(
    state: &crate::Transit,
    scope: &Scope,
    route_id: &str,
    direction: i16,
    shape_id: Option<&str>,
    stop: Option<&str>,
  ) -> crate::Result<Self> {
    let shape = match shape_id {
      Some(id) => RouteShape::load_by_shape(state, scope, id).await?,
      None => RouteShape::load(state, scope, route_id, direction).await?,
    };
    let variant = shape.shape_id.as_deref().or(shape.headsign.as_deref()).unwrap_or("");
    let rest =
      format!("routes:times:{}:{route_id}:{}:{variant}:{}", scope.provider, shape.direction_id, stop.unwrap_or(""));
    let key = crate::cache::Cache::key(scope.region.id(), &rest);
    state.cache.get_or_load(&key, Self::TTL, Self::get(&state.db, scope, &shape, stop)).await
  }
}

impl RouteSchedule {
  /// Defaults to today, expires at midnight
  pub async fn load(
    state: &crate::Transit,
    scope: &Scope,
    route_id: &str,
    direction: i16,
    stop: &str,
    day: Option<DayType>,
  ) -> crate::Result<Self> {
    let now = chrono::Local::now();
    let today = now.date_naive();
    let day = day.unwrap_or(DayType::of(today));
    let date = day.next_date(today);

    let cache = &state.cache;
    let rest = format!("routes:schedule:{}:{route_id}:{direction}:{stop}:{date}", scope.provider);
    let key = crate::cache::Cache::key(scope.region.id(), &rest);
    let ttl = crate::cache::Cache::until_midnight().min(cache.until_sync());
    let mut schedule =
      cache.get_or_load(&key, ttl, Self::get(&state.db, scope, route_id, direction, stop, day, date)).await?;

    if date == today {
      let elapsed = now.num_seconds_from_midnight() as i32 - 60;
      let next = schedule.hours.iter_mut().flat_map(|h| h.departures.iter_mut()).find(|d| d.secs >= elapsed);
      if let Some(d) = next {
        d.next = true;
      }
    }
    Ok(schedule)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn day_type_picks_today_or_the_next_one() {
    let tuesday = NaiveDate::from_ymd_opt(2026, 11, 3).unwrap();
    assert_eq!(DayType::Workday.next_date(tuesday), tuesday);
    assert_eq!(DayType::Saturday.next_date(tuesday), NaiveDate::from_ymd_opt(2026, 11, 7).unwrap());
    assert_eq!(DayType::Sunday.next_date(tuesday), NaiveDate::from_ymd_opt(2026, 11, 8).unwrap());
    let saturday = NaiveDate::from_ymd_opt(2026, 11, 7).unwrap();
    assert_eq!(DayType::Workday.next_date(saturday), NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());
  }
}
