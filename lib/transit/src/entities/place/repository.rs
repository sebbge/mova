use super::{Place, corner, normalize};
use crate::sdk::geo::Point;

impl Place {
  const NAMED_LIMIT: i64 = 8;
  const CORNER_LIMIT: i64 = 4;
  const CORNER_REACH_M: f64 = 120.0;
  const ZONE_REACH_M: f64 = 3000.0;

  pub async fn search(transit: &crate::Transit, term: &str, near: Option<Point>) -> crate::Result<Vec<Self>> {
    let words = patterns(&normalize(term));
    if words.is_empty() {
      return Ok(Vec::new());
    }

    let (corners, named) = tokio::join!(Self::corners(&transit.db, term, near), Self::named(&transit.db, &words, near));
    let mut hits = corners?;
    hits.extend(named?);
    Ok(hits)
  }

  async fn named(db: &crate::Db, words: &[String], near: Option<Point>) -> crate::Result<Vec<Self>> {
    let q = r#"
    SELECT id, kind, name, category, context,
           round(ST_Y(geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(geom::geometry)::numeric, 5)::float8 AS lng, area
    FROM places
    WHERE kind <> 'corner' AND key LIKE $1 AND key LIKE ALL($2::text[])
    ORDER BY rank,
             CASE WHEN $3::float8 IS NULL THEN 0
                  ELSE ST_Distance(geom, ST_SetSRID(ST_MakePoint($4::float8, $3::float8), 4326)::geography) END,
             name
    LIMIT $5
    "#;
    Ok(
      sqlx::query_as(q)
        .bind(longest(words))
        .bind(words)
        .bind(near.map(|p| p.lat))
        .bind(near.map(|p| p.lng))
        .bind(Self::NAMED_LIMIT)
        .fetch_all(db)
        .await?,
    )
  }

  async fn corners(db: &crate::Db, term: &str, near: Option<Point>) -> crate::Result<Vec<Self>> {
    let Some((a, b)) = corner(term) else {
      return Ok(Vec::new());
    };
    let (a, b) = (patterns(&a), patterns(&b));
    let all: Vec<String> = a.iter().chain(&b).cloned().collect();

    let q = r#"
    SELECT id, kind, name, category, context,
           round(ST_Y(geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(geom::geometry)::numeric, 5)::float8 AS lng, area
    FROM places
    WHERE kind = 'corner'
      AND (key LIKE $3 OR crossing LIKE $3)
      AND ((key LIKE ALL($1::text[]) AND crossing LIKE ALL($2::text[]))
        OR (key LIKE ALL($2::text[]) AND crossing LIKE ALL($1::text[])))
    ORDER BY CASE WHEN $4::float8 IS NULL THEN 0
                  ELSE ST_Distance(geom, ST_SetSRID(ST_MakePoint($5::float8, $4::float8), 4326)::geography) END,
             name
    LIMIT $6
    "#;
    Ok(
      sqlx::query_as(q)
        .bind(&a)
        .bind(&b)
        .bind(longest(&all))
        .bind(near.map(|p| p.lat))
        .bind(near.map(|p| p.lng))
        .bind(Self::CORNER_LIMIT)
        .fetch_all(db)
        .await?,
    )
  }

  pub async fn load(transit: &crate::Transit, id: &str) -> crate::Result<Self> {
    let q = r#"
    SELECT id, kind, name, category, context,
           round(ST_Y(geom::geometry)::numeric, 5)::float8 AS lat,
           round(ST_X(geom::geometry)::numeric, 5)::float8 AS lng, area
    FROM places WHERE id = $1
    "#;
    let place: Option<Self> = sqlx::query_as(q).bind(id).fetch_optional(&transit.db).await?;
    place.ok_or_else(|| crate::Error::NotFound(format!("place {id} not found")))
  }

  /// Nearest corner, else the zone
  pub async fn reverse(transit: &crate::Transit, at: Point) -> crate::Result<Option<Self>> {
    let q = r#"
    WITH p AS (SELECT ST_SetSRID(ST_MakePoint($2::float8, $1::float8), 4326)::geography AS g)
    SELECT id, kind, name, category, context, lat, lng, area FROM (
      (SELECT id, kind, name, category, context, geom, area, 0 AS pref FROM places, p
       WHERE kind = 'corner' AND ST_DWithin(geom, p.g, $3) ORDER BY geom <-> p.g LIMIT 1)
      UNION ALL
      (SELECT id, kind, name, category, context, geom, area, 1 AS pref FROM places, p
       WHERE kind = 'zone' AND ST_DWithin(geom, p.g, $4) ORDER BY geom <-> p.g LIMIT 1)
    ) r
    CROSS JOIN LATERAL (SELECT round(ST_Y(r.geom::geometry)::numeric, 5)::float8 AS lat,
                               round(ST_X(r.geom::geometry)::numeric, 5)::float8 AS lng) c
    ORDER BY pref
    LIMIT 1
    "#;
    Ok(
      sqlx::query_as(q)
        .bind(at.lat)
        .bind(at.lng)
        .bind(Self::CORNER_REACH_M)
        .bind(Self::ZONE_REACH_M)
        .fetch_optional(&transit.db)
        .await?,
    )
  }
}

fn patterns(normalized: &str) -> Vec<String> {
  normalized.split(' ').filter(|w| !w.is_empty()).map(|w| format!("%{w}%")).collect()
}

fn longest(patterns: &[String]) -> &str {
  patterns.iter().max_by_key(|p| p.len()).map_or("%", String::as_str)
}
