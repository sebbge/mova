use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

pub mod bus;
pub mod health;
pub mod line;
pub mod place;
pub mod plan;
pub mod routes;
pub mod stop;

use crate::transit::entities;

pub fn point(raw: &str) -> crate::Result<crate::sdk::geo::Point> {
  let invalid = || crate::Error::BadRequest(format!("invalid point: {raw}"));
  let (lat, lng) = raw.split_once(',').ok_or_else(invalid)?;
  let (lat, lng): (f64, f64) = (lat.trim().parse().map_err(|_| invalid())?, lng.trim().parse().map_err(|_| invalid())?);
  if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
    return Err(invalid());
  }
  Ok(crate::sdk::geo::Point { lat, lng })
}
