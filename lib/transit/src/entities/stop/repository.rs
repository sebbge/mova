use super::super::bus::Bus;
use super::{
  ApproachingBusRow, PastStopBusRow, ScheduleProfile, ScheduledLine, ServiceKey, Stop, StopArrival, StopDetail,
  StopHit, StopItem, StopLine, StopLines, StopRoute, StopSchedule,
};
use crate::helpers::Encoding;
use crate::scope::Scope;

use std::collections::HashMap;

impl Stop {
  pub async fn name(db: &crate::Db, scope: &Scope, stop_id: &str) -> crate::Result<Option<String>> {
    let q = "SELECT stop_name FROM stops WHERE region = $1 AND provider = $2 AND stop_id = $3";
    Ok(sqlx::query_scalar(q).bind(scope.region.id()).bind(scope.provider).bind(stop_id).fetch_optional(db).await?)
  }
}

impl StopLine {
  pub async fn get(db: &crate::Db, scope: &Scope, stop_id: &str) -> crate::Result<StopLines> {
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
      zonal: bool,
      interpolated: bool,
    }

    let q = r#"
    WITH here AS MATERIALIZED (
      SELECT st.*, t.route_id, t.service_id, t.trip_headsign FROM stop_times st
      JOIN trips t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      WHERE st.region = $3 AND st.provider = $4 AND st.stop_id = $2
        AND EXISTS (SELECT 1 FROM stop_times nx WHERE nx.region = st.region AND nx.provider = st.provider
                    AND nx.trip_id = st.trip_id AND nx.stop_sequence > st.stop_sequence)
    ),
    all_departures AS (
        -- Today's normal trips (departure_time up to 24:00:00)
        SELECT
            r.route_short_name                                             AS line,
            st.trip_headsign                                                AS destination,
            EXTRACT(EPOCH FROM (st.departure_time - $1::interval))::bigint AS secs,
            st.zonal, st.interpolated
        FROM here        st
        JOIN routes      r  ON r.region = st.region AND r.provider = st.provider AND r.route_id = st.route_id
        JOIN calendar    c  ON c.region = st.region AND c.provider = st.provider AND c.service_id = st.service_id
        WHERE st.departure_time > $1::interval
          AND st.departure_time <= INTERVAL '1 day'
          AND c.start_date <= $5::date
          AND c.end_date   >= $5::date
          AND CASE EXTRACT(DOW FROM $5::date)::int
                WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
                WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
                WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
                WHEN 0 THEN c.sunday
              END = true

        UNION ALL

        -- Yesterday's overnight trips (departure_time > 24:00:00)
        SELECT
            r.route_short_name                                                             AS line,
            st.trip_headsign                                                                AS destination,
            EXTRACT(EPOCH FROM (st.departure_time - INTERVAL '1 day' - $1::interval))::bigint AS secs,
            st.zonal, st.interpolated
        FROM here        st
        JOIN routes      r  ON r.region = st.region AND r.provider = st.provider AND r.route_id = st.route_id
        JOIN calendar    c  ON c.region = st.region AND c.provider = st.provider AND c.service_id = st.service_id
        WHERE st.departure_time > INTERVAL '1 day'
          AND st.departure_time - INTERVAL '1 day' > $1::interval
          AND c.start_date <= $5::date - 1
          AND c.end_date   >= $5::date - 1
          AND CASE EXTRACT(DOW FROM $5::date - 1)::int
                WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
                WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
                WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
                WHEN 0 THEN c.sunday
              END = true
    ),
    ranked AS (
        SELECT
            line, destination, secs, zonal, interpolated,
            ROW_NUMBER() OVER (
                PARTITION BY line, UPPER(TRIM(destination))
                ORDER BY secs
            ) AS rn
        FROM all_departures
    )
    SELECT line, destination, secs, rn, zonal, interpolated
    FROM ranked
    WHERE rn <= 3
    ORDER BY line, rn
    "#;
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(format!("{} seconds", today_secs))
      .bind(stop_id)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(now.date_naive())
      .fetch_all(db)
      .await?;

    let mut map = StopLines::new();
    for row in rows {
      let eta = row.secs.map(|s| chrono::Duration::seconds(s).num_minutes());
      let key = ServiceKey::new(&row.line, row.destination.as_deref());
      let entry = map.entry(key).or_insert_with(|| StopLine {
        line: row.line,
        destination: row.destination,
        company: String::new(),
        arrivals: vec![],
      });
      entry.arrivals.push(StopArrival {
        eta,
        live: false,
        bus: None,
        zonal: row.zonal,
        interpolated: row.interpolated,
        stop_position: None,
        source: super::ArrivalSource::Schedule,
      });
    }

    // roster-only lines, no arrivals
    let q = r#"
    SELECT line FROM stop_lines
    WHERE region = $2 AND provider = $3 AND stop_id = $1
    "#;
    let roster: Vec<String> =
      sqlx::query_scalar(q).bind(stop_id).bind(scope.region.id()).bind(scope.provider).fetch_all(db).await?;

    for line in roster {
      if map.values().any(|l| l.line == line) {
        continue;
      }
      map.insert(
        ServiceKey::new(&line, None),
        StopLine { line, destination: None, company: String::new(), arrivals: vec![] },
      );
    }

    let mut ids: Vec<&str> = map.values().map(|l| l.line.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    // shared lines (CE1) have no company
    let q = r#"
    SELECT line, min(company) FROM (
      SELECT line, company, seen_at, max(seen_at) OVER (PARTITION BY line) AS newest
      FROM line_companies
      WHERE region = $1 AND provider = $2 AND line = ANY($3)
    ) c
    WHERE seen_at > newest - INTERVAL '1 day'
    GROUP BY line
    HAVING count(*) = 1
    "#;
    let companies: HashMap<String, String> = sqlx::query_as(q)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(&ids)
      .fetch_all(db)
      .await?
      .into_iter()
      .collect();

    let mut lines: Vec<StopLine> = map.into_values().collect();
    for l in &mut lines {
      if let Some(company) = companies.get(&l.line) {
        l.company.clone_from(company);
      }
    }
    lines.sort_by(|a, b| a.cmp_by_first_eta(b));
    lines.truncate(20);
    Ok(lines.into_iter().map(|l| (l.key(), l)).collect())
  }

  /// Most frequent headsign per shape
  pub async fn shape_headsigns(
    db: &crate::Db,
    scope: &Scope,
    shape_ids: &[String],
  ) -> crate::Result<HashMap<String, String>> {
    if shape_ids.is_empty() {
      return Ok(HashMap::new());
    }
    let q = r#"
    SELECT DISTINCT ON (shape_id) shape_id, trip_headsign FROM trips
    WHERE region = $1 AND provider = $2 AND shape_id = ANY($3) AND trip_headsign IS NOT NULL
    GROUP BY shape_id, trip_headsign
    ORDER BY shape_id, count(*) DESC
    "#;
    let rows: Vec<(String, String)> =
      sqlx::query_as(q).bind(scope.region.id()).bind(scope.provider).bind(shape_ids).fetch_all(db).await?;
    Ok(rows.into_iter().collect())
  }

  /// Lines without any shape
  pub async fn shapeless(db: &crate::Db, scope: &Scope, lines: &[String]) -> crate::Result<Vec<String>> {
    let q = r#"
    SELECT DISTINCT r.route_short_name FROM routes r
    WHERE r.region = $1 AND r.provider = $2 AND r.route_short_name = ANY($3)
      AND NOT EXISTS (SELECT 1 FROM trips t WHERE t.region = r.region AND t.provider = r.provider
                      AND t.route_id = r.route_id AND t.shape_id IS NOT NULL)
    "#;
    Ok(sqlx::query_scalar(q).bind(scope.region.id()).bind(scope.provider).bind(lines).fetch_all(db).await?)
  }
}

//- sql
impl StopSchedule {
  pub async fn get(db: &crate::Db, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    use chrono::Local;

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      destination: Option<String>,
      secs: i64,
      zonal: bool,
      interpolated: bool,
    }

    // yesterday past 24 h
    let q = r#"
    WITH here AS MATERIALIZED (
      SELECT st.*, t.route_id, t.service_id, t.trip_headsign FROM stop_times st
      JOIN trips t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      WHERE st.region = $2 AND st.provider = $3 AND st.stop_id = $1
        AND EXISTS (SELECT 1 FROM stop_times nx WHERE nx.region = st.region AND nx.provider = st.provider
                    AND nx.trip_id = st.trip_id AND nx.stop_sequence > st.stop_sequence)
    )
    SELECT r.route_short_name AS line,
           st.trip_headsign     AS destination,
           EXTRACT(EPOCH FROM st.departure_time)::bigint AS secs, st.zonal, st.interpolated
    FROM here st
    JOIN routes   r ON r.region = st.region AND r.provider = st.provider AND r.route_id = st.route_id
    JOIN calendar c ON c.region = st.region AND c.provider = st.provider AND c.service_id = st.service_id
    WHERE st.departure_time <= INTERVAL '1 day'
      AND c.start_date <= $4::date
      AND c.end_date   >= $4::date
      AND CASE EXTRACT(DOW FROM $4::date)::int
            WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
            WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
            WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
            WHEN 0 THEN c.sunday
          END = true

    UNION ALL

    SELECT r.route_short_name AS line,
           st.trip_headsign     AS destination,
           EXTRACT(EPOCH FROM st.departure_time - INTERVAL '1 day')::bigint AS secs, st.zonal, st.interpolated
    FROM here st
    JOIN routes   r ON r.region = st.region AND r.provider = st.provider AND r.route_id = st.route_id
    JOIN calendar c ON c.region = st.region AND c.provider = st.provider AND c.service_id = st.service_id
    WHERE st.departure_time > INTERVAL '1 day'
      AND c.start_date <= $4::date - 1
      AND c.end_date   >= $4::date - 1
      AND CASE EXTRACT(DOW FROM $4::date - 1)::int
            WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
            WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
            WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
            WHEN 0 THEN c.sunday
          END = true
    "#;
    let now = Local::now();
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(stop_id)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(now.date_naive())
      .fetch_all(db)
      .await?;

    /// Departures attributed by us
    #[derive(Default)]
    struct Acc {
      departures: Vec<i64>,
      destinations: HashMap<String, usize>,
      zonal_count: usize,
      interpolated_count: usize,
    }

    let mut by_line: HashMap<String, Acc> = HashMap::new();
    for row in rows {
      let entry = by_line.entry(row.line).or_default();
      entry.departures.push(row.secs);
      if let Some(destination) = row.destination {
        *entry.destinations.entry(destination).or_default() += 1;
      }
      if row.zonal {
        entry.zonal_count += 1;
      }
      if row.interpolated {
        entry.interpolated_count += 1;
      }
    }

    let mut lines: Vec<ScheduledLine> = by_line
      .into_iter()
      .map(|(line, mut acc)| {
        acc.departures.sort_unstable();
        let destination = acc.destinations.into_iter().max_by_key(|(_, n)| *n).map(|(d, _)| d);
        let zonal = acc.zonal_count == acc.departures.len();
        let interpolated = acc.interpolated_count == acc.departures.len();
        ScheduledLine { line, destination, departures: acc.departures, zonal, interpolated }
      })
      .collect();

    lines.sort_by(|a, b| crate::helpers::cmp_line(&a.line, &b.line));

    Ok(Self { service_date: now.format("%Y-%m-%d").to_string(), lines })
  }
}

//- sql
impl Stop {
  pub async fn get(db: &crate::Db, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    #[derive(sqlx::FromRow)]
    struct StopRow {
      stop_id: String,
      stop_name: String,
      area: Option<String>,
      lat: f64,
      lng: f64,
      bearing_rad: Option<f64>,
    }

    let q = r#"
    SELECT s.stop_id, s.stop_name, s.area, s.bearing_rad,
           round(ST_Y(s.geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(s.geom::geometry)::numeric, 5)::float8 AS lng
    FROM stops s WHERE s.region = $2 AND s.provider = $3 AND s.stop_id = $1
    "#;
    let row: Option<StopRow> =
      sqlx::query_as(q).bind(stop_id).bind(scope.region.id()).bind(scope.provider).fetch_optional(db).await?;
    let row = row.ok_or_else(|| crate::Error::NotFound(format!("stop {stop_id} not found")))?;

    #[derive(sqlx::FromRow)]
    struct RouteRow {
      line: String,
      destination: Option<String>,
    }

    // natural line order, destinations by frequency
    // roster lines, no destination
    let q = r#"
    WITH here AS MATERIALIZED (
      SELECT t.region, t.provider, t.route_id, t.trip_headsign FROM stop_times st
      JOIN trips t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      WHERE st.region = $2 AND st.provider = $3 AND st.stop_id = $1
        AND EXISTS (SELECT 1 FROM stop_times nx WHERE nx.region = st.region AND nx.provider = st.provider
                    AND nx.trip_id = st.trip_id AND nx.stop_sequence > st.stop_sequence)
    ),
    scheduled AS (
      SELECT r.route_short_name AS line, a.trip_headsign AS destination, COUNT(*) AS n
      FROM here a
      JOIN routes r ON r.region = a.region AND r.provider = a.provider AND r.route_id = a.route_id
      GROUP BY r.route_short_name, a.trip_headsign
    ),
    roster_only AS (
      SELECT sl.line, NULL::text AS destination, 0::bigint AS n
      FROM stop_lines sl
      WHERE sl.region = $2 AND sl.provider = $3 AND sl.stop_id = $1
        AND NOT EXISTS (SELECT 1 FROM scheduled ch WHERE ch.line = sl.line)
    )
    SELECT line, destination FROM (
      SELECT * FROM scheduled UNION ALL SELECT * FROM roster_only
    ) u
    ORDER BY NULLIF(regexp_replace(line, '\D', '', 'g'), '')::bigint NULLS LAST,
             line,
             n DESC
    "#;
    let rows: Vec<RouteRow> =
      sqlx::query_as(q).bind(stop_id).bind(scope.region.id()).bind(scope.provider).fetch_all(db).await?;

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

    // leading stop first
    let q = r#"
    WITH this_stop AS (
      SELECT region, COALESCE(cluster, provider || '-' || stop_id) AS grp
      FROM stops WHERE region = $2 AND provider = $3 AND stop_id = $1
    )
    SELECT s.provider || '-' || s.stop_id AS reference, s.stop_name, s.station, s.platform
    FROM stops s JOIN this_stop e ON s.region = e.region
    WHERE COALESCE(s.cluster, s.provider || '-' || s.stop_id) = e.grp
    ORDER BY (s.provider || '-' || s.stop_id = e.grp) DESC, s.provider
    "#;

    #[derive(sqlx::FromRow)]
    struct SiblingRow {
      reference: String,
      stop_name: String,
      station: Option<String>,
      platform: Option<String>,
    }

    let group: Vec<SiblingRow> =
      sqlx::query_as(q).bind(stop_id).bind(scope.region.id()).bind(scope.provider).fetch_all(db).await?;

    // shelter named after the leading stop
    let (stop_name, station, platform) = match group.first() {
      Some(s) => (s.stop_name.clone(), s.station.clone(), s.platform.clone()),
      None => (row.stop_name, None, None),
    };
    let siblings: Vec<String> = group.into_iter().map(|s| s.reference).collect();

    Ok(Self {
      stop_id: crate::scope::qualify(scope.provider, &row.stop_id),
      stop_name,
      station,
      platform,
      area: row.area,
      lat: row.lat,
      lng: row.lng,
      bearing_rad: row.bearing_rad,
      siblings,
      lines,
    })
  }

  /// 5 decimals (~1 m), by area not region
  pub async fn list(db: &crate::Db, area: &str) -> crate::Result<Vec<StopItem>> {
    // area is nullable
    let q = r#"
    SELECT s.provider || '-' || s.stop_id AS id, s.stop_name AS name,
           round(ST_Y(s.geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(s.geom::geometry)::numeric, 5)::float8 AS lng,
           g.providers, s.station, s.platform, f.footprint
    FROM stops s
    LEFT JOIN LATERAL (
      SELECT array_agg(DISTINCT o.provider) AS providers
      FROM stops o
      WHERE o.region = s.region AND o.cluster = s.cluster
    ) g ON s.cluster IS NOT NULL
    LEFT JOIN LATERAL (
      SELECT ARRAY(
        SELECT round(c::numeric, 5)::float8
        FROM ST_DumpPoints(ST_ExteriorRing(t.geom)) d,
             unnest(ARRAY[ST_X(d.geom), ST_Y(d.geom)]) WITH ORDINALITY AS u(c, i)
        ORDER BY d.path, u.i
      ) AS footprint
      FROM terminals t
      WHERE t.region = s.region AND t.provider = s.provider AND t.stop_id = s.stop_id
      ORDER BY ST_Area(t.geom) DESC
      LIMIT 1
    ) f ON true
    WHERE COALESCE(s.area, s.region) = $1
      -- shared shelter: leader only
      AND (s.cluster IS NULL OR s.cluster = s.provider || '-' || s.stop_id)
      -- zone markers hidden once their schedules moved
      AND (NOT s.zonal OR EXISTS (
        SELECT 1 FROM stop_times t
        WHERE t.region = s.region AND t.provider = s.provider AND t.stop_id = s.stop_id))
    ORDER BY s.stop_id
    "#;
    Ok(sqlx::query_as(q).bind(area).fetch_all(db).await?)
  }

  pub async fn load_search(
    transit: &crate::Transit,
    term: &str,
    near: Option<crate::sdk::geo::Point>,
  ) -> crate::Result<Vec<StopHit>> {
    let q = r#"
    WITH words AS (
      SELECT w FROM unnest(string_to_array(translate(lower($1), 'áéíóúüñ', 'aeiouun'), ' ')) AS w WHERE w <> ''
    )
    SELECT s.provider || '-' || s.stop_id AS id, s.stop_name AS name,
           round(ST_Y(s.geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(s.geom::geometry)::numeric, 5)::float8 AS lng,
           COALESCE(s.area, s.region) AS area, s.station, s.platform
    FROM stops s
    WHERE (s.cluster IS NULL OR s.cluster = s.provider || '-' || s.stop_id)
      AND NOT s.zonal
      AND EXISTS (SELECT 1 FROM words)
      AND NOT EXISTS (
        SELECT 1 FROM words
        WHERE translate(lower(s.stop_name || ' ' || COALESCE(s.station, '')), 'áéíóúüñ', 'aeiouun') NOT LIKE '%' || w || '%'
      )
    ORDER BY CASE WHEN $2::float8 IS NULL THEN 0
                  ELSE ST_Distance(s.geom, ST_SetSRID(ST_MakePoint($3::float8, $2::float8), 4326)::geography) END,
             s.stop_name
    LIMIT 12
    "#;
    Ok(sqlx::query_as(q).bind(term).bind(near.map(|p| p.lat)).bind(near.map(|p| p.lng)).fetch_all(&transit.db).await?)
  }

  /// Terminal outline to its "Terminal …" or "Intercambiador …" stop nearest the center, returns the count
  pub async fn rebuild_terminals(db: &crate::Db, reach_m: f64) -> crate::Result<u64> {
    sqlx::query("UPDATE terminals SET region = NULL, provider = NULL, stop_id = NULL").execute(db).await?;

    let q = r#"
    UPDATE terminals t SET region = d.region, provider = d.provider, stop_id = d.stop_id
    FROM (
      SELECT DISTINCT ON (t.osm_id) t.osm_id, s.region, s.provider, s.stop_id
      FROM terminals t
      JOIN stops s ON ST_DWithin(s.geom, t.geom::geography, $1)
      WHERE NOT s.zonal AND (s.cluster IS NULL OR s.cluster = s.provider || '-' || s.stop_id)
      ORDER BY t.osm_id, (s.station IS NOT NULL OR s.stop_name ~* '^(terminal|intercambiador)\M') DESC,
               ST_Distance(s.geom, ST_Centroid(t.geom)::geography)
    ) d
    WHERE d.osm_id = t.osm_id
    "#;
    Ok(sqlx::query(q).bind(reach_m).execute(db).await?.rows_affected())
  }

  /// Groups mutually nearest stops of different providers, returns the count
  pub async fn rebuild_clusters(db: &crate::Db, priority: &[String], radius_m: f64) -> crate::Result<u64> {
    sqlx::query("UPDATE stops SET cluster = NULL WHERE cluster IS NOT NULL").execute(db).await?;

    let q = r#"
    WITH prio AS (
      SELECT provider, rank FROM unnest($1::text[]) WITH ORDINALITY AS t(provider, rank)
    ),
    neighbor AS (
      SELECT DISTINCT ON (a.region, a.provider, a.stop_id)
             a.region, a.provider AS ap, a.stop_id AS aid, b.provider AS bp, b.stop_id AS bid
      FROM stops a
      JOIN stops b
        ON b.region = a.region
       AND b.provider <> a.provider
       AND NOT b.zonal
       AND ST_DWithin(a.geom, b.geom, $2)
      JOIN prio pa ON pa.provider = a.provider
      JOIN prio pb ON pb.provider = b.provider
      -- perpendicular to the leader's heading is the opposite sidewalk
      -- angle ignored under 10 m
      CROSS JOIN LATERAL (
        SELECT CASE WHEN pa.rank < pb.rank THEN a.bearing_rad ELSE b.bearing_rad END AS heading,
               CASE WHEN pa.rank < pb.rank THEN ST_Azimuth(a.geom, b.geom) ELSE ST_Azimuth(b.geom, a.geom) END AS hacia
      ) m
      WHERE ST_Distance(a.geom, b.geom) <= 10 OR m.heading IS NULL
         OR degrees(abs(atan2(sin(m.hacia - m.heading), cos(m.hacia - m.heading)))) NOT BETWEEN 60 AND 120
      ORDER BY a.region, a.provider, a.stop_id, ST_Distance(a.geom, b.geom), b.provider, b.stop_id
    ),
    mutual AS (
      SELECT v.* FROM neighbor v
      JOIN neighbor w
        ON w.region = v.region AND w.ap = v.bp AND w.aid = v.bid
       AND w.bp = v.ap AND w.bid = v.aid
    ),
    leader AS (
      SELECT m.region, m.ap, m.aid,
             CASE WHEN pa.rank < pb.rank THEN m.ap || '-' || m.aid ELSE m.bp || '-' || m.bid END AS cluster
      FROM mutual m
      JOIN prio pa ON pa.provider = m.ap
      JOIN prio pb ON pb.provider = m.bp
    )
    UPDATE stops s SET cluster = d.cluster
    FROM leader d
    WHERE s.region = d.region AND s.provider = d.ap AND s.stop_id = d.aid
    "#;

    let done = sqlx::query(q).bind(priority).bind(radius_m).execute(db).await?;
    Ok(done.rows_affected())
  }

  /// Moves zone marker schedules to real stops by side of the road, returns the count
  pub async fn absorb_zonal_schedules(db: &crate::Db, radius_m: f64) -> crate::Result<u64> {
    let q = r#"
    WITH candidate_pairs AS (
      SELECT DISTINCT z.region, z.provider, z.stop_id AS zid, t.shape_id
      FROM stops z
      JOIN stop_times st ON st.region = z.region AND st.provider = z.provider AND st.stop_id = z.stop_id
      JOIN trips t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      WHERE z.zonal AND t.shape_id IS NOT NULL
    ),
    chosen AS (
      SELECT DISTINCT ON (p.region, p.provider, p.zid, p.shape_id)
             p.region, p.provider, p.zid, p.shape_id, c.stop_id AS cid
      FROM candidate_pairs p
      JOIN stops z  ON z.region = p.region AND z.provider = p.provider AND z.stop_id = p.zid
      JOIN shapes sh ON sh.region = p.region AND sh.provider = p.provider AND sh.shape_id = p.shape_id
      JOIN stops c  ON c.region = p.region AND c.provider = p.provider
                   AND NOT c.zonal AND ST_DWithin(c.geom, z.geom, $1)
      CROSS JOIN LATERAL (
        SELECT ST_Azimuth(
          ST_LineInterpolatePoint(sh.geom, greatest(ST_LineLocatePoint(sh.geom, c.geom::geometry)
                                                    - 20.0 / ST_Length(sh.geom::geography), 0)),
          ST_LineInterpolatePoint(sh.geom, least(ST_LineLocatePoint(sh.geom, c.geom::geometry)
                                                 + 20.0 / ST_Length(sh.geom::geography), 1))) AS heading
      ) tg
      WHERE degrees(atan2(
              sin(ST_Azimuth(ST_ClosestPoint(sh.geom, c.geom::geometry), c.geom::geometry) - tg.heading),
              cos(ST_Azimuth(ST_ClosestPoint(sh.geom, c.geom::geometry), c.geom::geometry) - tg.heading)
            )) BETWEEN 0 AND 180
      ORDER BY p.region, p.provider, p.zid, p.shape_id, ST_Distance(c.geom, z.geom)
    ),
    crossed AS (
      DELETE FROM stop_times o
      USING chosen e, trips t
      WHERE o.region = e.region AND o.provider = e.provider AND o.stop_id = e.cid AND o.interpolated
        AND t.region = o.region AND t.provider = o.provider AND t.trip_id = o.trip_id AND t.shape_id = e.shape_id
        AND EXISTS (SELECT 1 FROM stop_times m WHERE m.region = o.region AND m.provider = o.provider
                    AND m.trip_id = o.trip_id AND m.stop_id = e.zid)
    )
    UPDATE stop_times st SET stop_id = e.cid, zonal = TRUE
    FROM chosen e, trips t
    WHERE t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      AND t.shape_id = e.shape_id
      AND st.region = e.region AND st.provider = e.provider AND st.stop_id = e.zid
    "#;
    let mut moved = sqlx::query(q).bind(radius_m).execute(db).await?.rows_affected();

    // second pass, one-direction markers
    let q = r#"
    WITH one_direction AS (
      SELECT z.region, z.provider, z.stop_id
      FROM stops z
      JOIN stop_times st ON st.region = z.region AND st.provider = z.provider AND st.stop_id = z.stop_id
      JOIN trips t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
      WHERE z.zonal
      GROUP BY 1, 2, 3
      HAVING count(DISTINCT t.direction_id) = 1
    ),
    chosen AS (
      SELECT DISTINCT ON (u.region, u.provider, u.stop_id) u.region, u.provider, u.stop_id AS zid, c.stop_id AS cid
      FROM one_direction u
      JOIN stops z ON z.region = u.region AND z.provider = u.provider AND z.stop_id = u.stop_id
      JOIN stops c ON c.region = u.region AND c.provider = u.provider
                  AND NOT c.zonal AND ST_DWithin(c.geom, z.geom, $1)
      ORDER BY u.region, u.provider, u.stop_id, ST_Distance(c.geom, z.geom)
    ),
    crossed AS (
      DELETE FROM stop_times o
      USING chosen e
      WHERE o.region = e.region AND o.provider = e.provider AND o.stop_id = e.cid AND o.interpolated
        AND EXISTS (SELECT 1 FROM stop_times m WHERE m.region = o.region AND m.provider = o.provider
                    AND m.trip_id = o.trip_id AND m.stop_id = e.zid)
    )
    UPDATE stop_times st SET stop_id = e.cid, zonal = TRUE
    FROM chosen e
    WHERE st.region = e.region AND st.provider = e.provider AND st.stop_id = e.zid
    "#;
    moved += sqlx::query(q).bind(radius_m).execute(db).await?.rows_affected();

    Ok(moved)
  }

  /// Departments with stops
  pub async fn areas(db: &crate::Db) -> crate::Result<Vec<String>> {
    let q = "SELECT DISTINCT COALESCE(area, region) AS area FROM stops ORDER BY 1";
    Ok(sqlx::query_scalar(q).fetch_all(db).await?)
  }
}

//- cache
impl Stop {
  fn stop_key(scope: &Scope, stop_id: &str) -> String {
    crate::cache::key(scope.region.id(), &format!("stop:{}:{stop_id}", scope.provider))
  }

  pub async fn load(state: &crate::Transit, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    let cache = &state.cache;
    cache.get_or_load(&Self::stop_key(scope, stop_id), cache.until_sync(), Self::get(&state.db, scope, stop_id)).await
  }

  fn list_key(area: &str, enc: Encoding) -> String {
    crate::cache::key(area, &format!("stops:area{}", enc.suffix()))
  }

  pub async fn load_list(transit: &crate::Transit, area: &str, enc: Encoding) -> crate::Result<Vec<u8>> {
    use crate::cache::CacheBytes;
    if let Some(bytes) = transit.cache.get_bytes(&Self::list_key(area, enc)).await {
      return Ok(bytes);
    }
    Self::warm_list(&transit.db, &transit.cache, area)
      .await?
      .into_iter()
      .find_map(|(e, bytes)| (e == enc).then_some(bytes))
      .ok_or_else(|| crate::Error::External("missing requested encoding variant".into()))
  }

  /// All encodings, stored and returned
  pub(crate) async fn warm_list(
    db: &crate::Db,
    cache: &crate::Cache,
    area: &str,
  ) -> crate::Result<Vec<(Encoding, Vec<u8>)>> {
    use crate::cache::CacheBytes;

    let stops = Self::list(db, area).await?;
    let json =
      serde_json::to_vec(&stops).map_err(|e| crate::Error::External(format!("couldn't serialize stop list: {e}")))?;

    let variants = tokio::task::spawn_blocking(move || {
      Encoding::ALL
        .iter()
        .map(|&enc| crate::helpers::compress(&json, enc).map(|b| (enc, b)))
        .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| crate::Error::External(format!("stop list compression panicked: {e}")))?
    .map_err(|e| crate::Error::External(format!("couldn't compress stop list: {e}")))?;

    let ttl = cache.until_sync();
    for (enc, bytes) in &variants {
      cache.set_bytes(&Self::list_key(area, *enc), bytes, ttl).await;
    }

    tracing::info!(
      area,
      stops = stops.len(),
      raw = variants[0].1.len(),
      gzip = variants[1].1.len(),
      brotli = variants[2].1.len(),
      "[stops] precompressed list"
    );
    Ok(variants)
  }
}

// cache
use crate::cache::CacheRw;

impl StopSchedule {
  fn schedule_key(scope: &Scope, stop_id: &str, date: &str) -> String {
    crate::cache::key(scope.region.id(), &format!("schedule:{}:{stop_id}:{date}", scope.provider))
  }

  /// Expires at midnight
  pub async fn load(state: &crate::Transit, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    let cache = &state.cache;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let ttl = crate::helpers::secs_to_midnight().min(cache.until_sync());
    cache.get_or_load(&Self::schedule_key(scope, stop_id, &today), ttl, Self::get(&state.db, scope, stop_id)).await
  }
}
impl StopDetail {
  const TTL: u64 = 10;

  fn stop_detail_key(scope: &Scope, stop_id: &str) -> String {
    crate::cache::key(scope.region.id(), &format!("stop_detail:{}:{stop_id}", scope.provider))
  }

  pub async fn load(state: &crate::Transit, scope: &Scope, stop_id: &str) -> crate::Result<Self> {
    state
      .cache
      .get_or_load(&Self::stop_detail_key(scope, stop_id), Self::TTL, Self::assemble(state, scope, stop_id))
      .await
  }
}

// bus-stop projection

/// Candidate shapes: from stop_times, else within 30 m
macro_rules! projection_cte {
  () => {
    r#"
      WITH target AS (
          SELECT s.geom,
                 COALESCE(s.bearing_rad,
                          (SELECT h.bearing_rad FROM stops h
                           WHERE h.region = s.region AND h.cluster = s.cluster AND h.bearing_rad IS NOT NULL
                           ORDER BY h.provider, h.stop_id LIMIT 1)) AS heading
          FROM stops s WHERE s.region = $7 AND s.provider = $8 AND s.stop_id = $5
      ),
      served AS (
          SELECT DISTINCT t.shape_id
          FROM stop_times st
          JOIN trips      t ON t.region = st.region AND t.provider = st.provider AND t.trip_id = st.trip_id
          WHERE st.region = $7 AND st.provider = $8 AND st.stop_id = $5
      ),
      shape_variants AS (
          SELECT DISTINCT r.route_short_name AS line, t.shape_id, t.trip_headsign AS headsign
          FROM routes r
          JOIN trips  t ON t.region = r.region AND t.provider = r.provider AND t.route_id = r.route_id
          WHERE r.region = $7 AND r.provider = $8 AND r.route_short_name = ANY($1::text[])
      ),
      -- geometry once per shape
      stop_shapes AS (
          SELECT v.line, v.shape_id, v.headsign,
              sh.geom AS shape,
              ST_LineLocatePoint(sh.geom, p.geom::geometry) AS stop_frac,
              v.shape_id IN (SELECT shape_id FROM served) AS serves
          FROM shape_variants v
          JOIN shapes sh ON sh.region = $7 AND sh.provider = $8 AND sh.shape_id = v.shape_id
          CROSS JOIN target p
          WHERE v.shape_id IN (SELECT shape_id FROM served)
             OR ST_DWithin(sh.geom::geography, p.geom::geography, 30)
      ),
      line_shapes AS (
          SELECT rc.*
          FROM stop_shapes rc
          CROSS JOIN target p
          WHERE rc.serves
             OR (NOT EXISTS (SELECT 1 FROM stop_shapes x WHERE x.line = rc.line AND x.serves)
                 -- TODO: without a heading of its own or a sibling, both directions enter
                 AND (p.heading IS NULL
                      OR cos(ST_Azimuth(
                               ST_LineInterpolatePoint(rc.shape,
                                 greatest(rc.stop_frac - 15.0 / ST_Length(rc.shape::geography), 0)),
                               ST_LineInterpolatePoint(rc.shape,
                                 least(rc.stop_frac + 15.0 / ST_Length(rc.shape::geography), 1)))
                             - p.heading) > 0))
      ),
      buses AS (
          SELECT *
          FROM unnest($1::text[], $2::float8[], $3::float8[], $4::text[], $6::text[], $9::text[])
            AS t(bus_line, bus_lat, bus_lng, bus_dest, bus_id, bus_shape)
      )
"#
  };
}

/// Realtime shape, else destination text
macro_rules! projection_match {
  () => {
    r#"
      JOIN line_shapes ls ON ls.line = b.bus_line
        AND CASE
              WHEN EXISTS (SELECT 1 FROM shapes x
                           WHERE x.region = $7 AND x.provider = $8 AND x.shape_id = b.bus_shape)
                THEN ls.shape_id = b.bus_shape
              ELSE b.bus_dest IS NULL
                   OR ls.headsign IS NULL
                   OR UPPER(TRIM(ls.headsign)) = UPPER(TRIM(b.bus_dest))
            END
"#
  };
}

impl StopArrival {
  /// Remaining distance of buses before the stop
  pub async fn query_approaching(
    db: &crate::Db,
    scope: &Scope,
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
    let mut p_shapes: Vec<Option<String>> = Vec::with_capacity(buses.len());

    for bus in buses {
      p_lines.push(bus.seen.line.clone());
      p_lats.push(bus.seen.location.lat);
      p_lngs.push(bus.seen.location.lng);
      p_dests.push(bus.seen.destination.clone());
      p_ids.push(Some(bus.seen.id.clone()));
      p_shapes.push(bus.seen.shape_id.clone());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      headsign: Option<String>,
      bus_id: Option<String>,
      remaining_m: Option<f64>,
      bus_frac: f64,
      stop_frac: f64,
      fracs: Option<Vec<f64>>,
      secs: Option<Vec<f64>>,
    }

    // reference trip in progress on the shape, today
    let q = concat!(
      projection_cte!(),
      r#"
      , nearby AS (
          SELECT DISTINCT ON (b.bus_id)
              b.bus_line AS line, ls.headsign, b.bus_id, ls.shape_id, ls.shape, ls.stop_frac,
              ST_LineLocatePoint(ls.shape,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) AS bus_frac
          FROM buses b"#,
      projection_match!(),
      r#"
          WHERE ST_LineLocatePoint(ls.shape,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) < ls.stop_frac
            AND ST_Distance(ls.shape::geography,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography) < 500
          ORDER BY b.bus_id,
              ST_Distance(ls.shape::geography,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography),
              UPPER(TRIM(ls.headsign)) IS NOT DISTINCT FROM UPPER(TRIM(b.bus_dest)) DESC
      )
      SELECT c.line, c.headsign, c.bus_id, c.bus_frac, c.stop_frac,
             ST_Length(ST_LineSubstring(c.shape, c.bus_frac, c.stop_frac)::geography) AS remaining_m,
             n.fracs, n.secs
      FROM nearby c
      LEFT JOIN LATERAL (
          SELECT tr.trip_id
          FROM trips tr
          WHERE tr.region = $7 AND tr.provider = $8 AND tr.shape_id = c.shape_id AND tr.starts IS NOT NULL
          ORDER BY
              EXISTS (
                  SELECT 1 FROM calendar k
                  WHERE k.region = tr.region AND k.provider = tr.provider AND k.service_id = tr.service_id
                    AND $11::date BETWEEN k.start_date AND k.end_date
                    AND CASE EXTRACT(DOW FROM $11::date)::int
                          WHEN 1 THEN k.monday    WHEN 2 THEN k.tuesday
                          WHEN 3 THEN k.wednesday WHEN 4 THEN k.thursday
                          WHEN 5 THEN k.friday    WHEN 6 THEN k.saturday
                          WHEN 0 THEN k.sunday
                        END
              ) DESC,
              ($10::interval BETWEEN tr.starts AND tr.ends
               OR $10::interval + INTERVAL '1 day' BETWEEN tr.starts AND tr.ends) DESC,
              abs(EXTRACT(EPOCH FROM tr.starts - $10::interval))
          LIMIT 1
      ) v ON true
      LEFT JOIN LATERAL (
          SELECT array_agg(ST_LineLocatePoint(c.shape, s.geom::geometry) ORDER BY st.stop_sequence) AS fracs,
                 array_agg(EXTRACT(EPOCH FROM st.departure_time)::float8 ORDER BY st.stop_sequence) AS secs
          FROM stop_times st
          JOIN stops s ON s.region = st.region AND s.provider = st.provider AND s.stop_id = st.stop_id
          WHERE st.region = $7 AND st.provider = $8 AND st.trip_id = v.trip_id
      ) n ON true
      "#
    );
    let now = chrono::Local::now();
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(&p_lines)
      .bind(&p_lats)
      .bind(&p_lngs)
      .bind(&p_dests)
      .bind(stop_id)
      .bind(&p_ids)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(&p_shapes)
      .bind(format!("{} seconds", chrono::Timelike::num_seconds_from_midnight(&now)))
      .bind(now.date_naive())
      .fetch_all(db)
      .await?;

    Ok(
      rows
        .into_iter()
        .filter_map(|r| {
          let remaining_m = r.remaining_m.filter(|&m| m > 0.0)?;
          let scheduled_s = match (&r.fracs, &r.secs) {
            (Some(fracs), Some(secs)) => ScheduleProfile::new(fracs, secs).between(r.bus_frac, r.stop_frac),
            _ => None,
          };
          Some(ApproachingBusRow {
            line: r.line,
            headsign: r.headsign,
            bus_id: r.bus_id.unwrap_or_default(),
            remaining_m,
            scheduled_s,
          })
        })
        .collect(),
    )
  }

  /// Distance (m) of buses past the stop, within a margin
  pub async fn query_past_stop(
    db: &crate::Db,
    scope: &Scope,
    stop_id: &str,
    buses: &[Bus],
  ) -> crate::Result<Vec<PastStopBusRow>> {
    if buses.is_empty() {
      return Ok(Vec::new());
    }

    let mut p_lines: Vec<String> = Vec::with_capacity(buses.len());
    let mut p_lats: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_lngs: Vec<f64> = Vec::with_capacity(buses.len());
    let mut p_dests: Vec<Option<String>> = Vec::with_capacity(buses.len());
    let mut p_ids: Vec<Option<String>> = Vec::with_capacity(buses.len());
    let mut p_shapes: Vec<Option<String>> = Vec::with_capacity(buses.len());

    for bus in buses {
      p_lines.push(bus.seen.line.clone());
      p_lats.push(bus.seen.location.lat);
      p_lngs.push(bus.seen.location.lng);
      p_dests.push(bus.seen.destination.clone());
      p_ids.push(Some(bus.seen.id.clone()));
      p_shapes.push(bus.seen.shape_id.clone());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
      line: String,
      headsign: Option<String>,
      bus_id: Option<String>,
      past_m: Option<f64>,
    }

    let q = concat!(
      projection_cte!(),
      r#"
      SELECT DISTINCT ON (b.bus_id)
          b.bus_line AS line, ls.headsign, b.bus_id,
          ST_Length(ST_LineSubstring(
              ls.shape,
              ls.stop_frac,
              ST_LineLocatePoint(ls.shape,
                  ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326))
          )::geography) AS past_m
      FROM buses b"#,
      projection_match!(),
      r#"
      WHERE ST_LineLocatePoint(ls.shape,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) >= ls.stop_frac
        AND ST_LineLocatePoint(ls.shape,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)) <= ls.stop_frac + 0.05
        AND ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography) < 500
      ORDER BY b.bus_id,
          ST_Distance(ls.shape::geography,
              ST_SetSRID(ST_MakePoint(b.bus_lng, b.bus_lat), 4326)::geography),
          UPPER(TRIM(ls.headsign)) IS NOT DISTINCT FROM UPPER(TRIM(b.bus_dest)) DESC
      "#
    );
    let rows: Vec<Row> = sqlx::query_as(q)
      .bind(&p_lines)
      .bind(&p_lats)
      .bind(&p_lngs)
      .bind(&p_dests)
      .bind(stop_id)
      .bind(&p_ids)
      .bind(scope.region.id())
      .bind(scope.provider)
      .bind(&p_shapes)
      .fetch_all(db)
      .await?;

    Ok(
      rows
        .into_iter()
        .map(|r| PastStopBusRow {
          line: r.line,
          headsign: r.headsign,
          bus_id: r.bus_id.unwrap_or_default(),
          past_m: r.past_m.unwrap_or(0.0),
        })
        .collect(),
    )
  }
}
