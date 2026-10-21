use super::{DayStop, DayStopTime, DayTrip, ServiceDay};
use crate::sdk::{Ref, geo::Point};

use chrono::NaiveDate;
use futures_util::TryStreamExt;

macro_rules! with_nodes {
  ($select:literal) => {
    concat!(
      r#"WITH
  days(shift, day) AS (
    SELECT -1, $1::date - 1 UNION ALL SELECT 0, $1::date UNION ALL SELECT 1, $1::date + 1
  ),
  active AS (
    SELECT (row_number() OVER (ORDER BY t.region, t.provider, t.trip_id, d.shift) - 1)::int AS idx,
           t.region, t.provider, t.trip_id, t.route_id, t.shape_id, t.trip_headsign, d.shift
    FROM trips t
    JOIN calendar c ON c.region = t.region AND c.provider = t.provider AND c.service_id = t.service_id
    JOIN days d ON c.start_date <= d.day AND c.end_date >= d.day
      AND CASE EXTRACT(DOW FROM d.day)::int
            WHEN 1 THEN c.monday    WHEN 2 THEN c.tuesday
            WHEN 3 THEN c.wednesday WHEN 4 THEN c.thursday
            WHEN 5 THEN c.friday    WHEN 6 THEN c.saturday
            WHEN 0 THEN c.sunday
          END
    WHERE d.shift = 0
       OR (d.shift = -1 AND t.ends > INTERVAL '1 day')
       OR (d.shift = 1 AND t.starts < INTERVAL '6 hours')
  ),
  nodes AS (
    SELECT (row_number() OVER (ORDER BY region, provider, stop_id) - 1)::int AS idx, region, provider, stop_id
    FROM stops
    WHERE NOT zonal
  )"#,
      $select
    )
  };
}

impl ServiceDay {
  pub async fn load(transit: &crate::Transit, date: NaiveDate) -> crate::Result<Self> {
    let mut tx = transit.tasks_db.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY").execute(&mut *tx).await?;

    let stops = Self::stops(&mut tx, date).await?;
    let mut trips = Self::trips(&mut tx, date).await?;
    Self::stop_times(&mut tx, date, &mut trips).await?;
    tx.commit().await?;

    trips.retain(|t| t.stop_times.len() >= 2);
    Ok(Self { date, stops, trips })
  }

  async fn stops(tx: &mut sqlx::PgConnection, date: NaiveDate) -> crate::Result<Vec<DayStop>> {
    #[derive(sqlx::FromRow)]
    struct Row {
      provider: String,
      stop_id: String,
      stop_name: String,
      lat: f64,
      lng: f64,
      cluster: String,
    }

    let q = with_nodes!(
      r#"
      SELECT s.provider, s.stop_id, s.stop_name, ST_Y(s.geom::geometry) AS lat, ST_X(s.geom::geometry) AS lng,
             COALESCE(s.cluster, s.provider || '-' || s.stop_id) AS cluster
      FROM nodes n
      JOIN stops s ON s.region = n.region AND s.provider = n.provider AND s.stop_id = n.stop_id
      ORDER BY n.idx"#
    );
    let rows: Vec<Row> = sqlx::query_as(q).bind(date).fetch_all(&mut *tx).await?;
    Ok(
      rows
        .into_iter()
        .map(|r| DayStop {
          id: Ref::new(r.provider, r.stop_id),
          name: r.stop_name,
          location: Point { lat: r.lat, lng: r.lng },
          cluster: r.cluster,
        })
        .collect(),
    )
  }

  async fn trips(tx: &mut sqlx::PgConnection, date: NaiveDate) -> crate::Result<Vec<DayTrip>> {
    #[derive(sqlx::FromRow)]
    struct Row {
      provider: String,
      route_id: String,
      line: String,
      trip_headsign: Option<String>,
      shape_id: Option<String>,
    }

    let q = with_nodes!(
      r#"
      SELECT a.provider, a.route_id, COALESCE(r.route_short_name, '') AS line, a.trip_headsign, a.shape_id
      FROM active a
      LEFT JOIN routes r ON r.region = a.region AND r.provider = a.provider AND r.route_id = a.route_id
      ORDER BY a.idx"#
    );
    let rows: Vec<Row> = sqlx::query_as(q).bind(date).fetch_all(&mut *tx).await?;
    Ok(
      rows
        .into_iter()
        .map(|r| DayTrip {
          route: Ref::new(r.provider, r.route_id),
          line: r.line,
          headsign: r.trip_headsign,
          shape_id: r.shape_id,
          stop_times: Vec::new(),
        })
        .collect(),
    )
  }

  async fn stop_times(tx: &mut sqlx::PgConnection, date: NaiveDate, trips: &mut [DayTrip]) -> crate::Result<()> {
    #[derive(sqlx::FromRow)]
    struct Row {
      trip: i32,
      stop: i32,
      stop_sequence: i32,
      arrival: i32,
      departure: i32,
      interpolated: bool,
    }

    sqlx::query("SET LOCAL enable_nestloop = off").execute(&mut *tx).await?;
    let q = with_nodes!(
      r#"
      SELECT a.idx AS trip, n.idx AS stop, st.stop_sequence,
             EXTRACT(EPOCH FROM st.arrival_time)::int + a.shift * 86400 AS arrival,
             EXTRACT(EPOCH FROM st.departure_time)::int + a.shift * 86400 AS departure,
             st.interpolated
      FROM active a
      JOIN stop_times st ON st.region = a.region AND st.provider = a.provider AND st.trip_id = a.trip_id
      JOIN nodes n ON n.region = st.region AND n.provider = st.provider AND n.stop_id = st.stop_id"#
    );

    let mut sequenced: Vec<Vec<(i32, DayStopTime)>> = vec![Vec::new(); trips.len()];
    let mut rows = sqlx::query_as::<_, Row>(q).bind(date).fetch(&mut *tx);
    while let Some(row) = rows.try_next().await? {
      let Some(times) = usize::try_from(row.trip).ok().and_then(|i| sequenced.get_mut(i)) else {
        continue;
      };
      let Ok(stop) = u32::try_from(row.stop) else {
        continue;
      };
      let time = DayStopTime { stop, arrival: row.arrival, departure: row.departure, interpolated: row.interpolated };
      times.push((row.stop_sequence, time));
    }

    for (trip, mut times) in trips.iter_mut().zip(sequenced) {
      times.sort_unstable_by_key(|(sequence, _)| *sequence);
      trip.stop_times = times.into_iter().map(|(_, time)| time).collect();
    }
    Ok(())
  }
}
