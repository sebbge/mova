use super::*;
use crate::planner::{Itinerary, Mode, Query as Trip, Rider};
use crate::sdk::fares::{Category, Payment};

use chrono::NaiveDateTime;

#[derive(serde::Deserialize)]
pub struct PlanQuery {
  from: String,
  to: String,
  at: Option<String>,
  payment: Option<Payment>,
  category: Option<Category>,
  mode: Option<Mode>,
}

///# GET <MOVA>/plan?from=-34.9,-56.16&to=-34.906,-56.2&at=2026-09-28T08:00&payment=card&category=common&mode=transit
pub async fn plan(State(state): State<crate::State>, Query(query): Query<PlanQuery>) -> crate::Res<Vec<Itinerary>> {
  let from = point(&query.from)?;
  let to = point(&query.to)?;
  let at = match query.at.as_deref() {
    Some(raw) => NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M")
      .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
      .map_err(|_| crate::Error::BadRequest(format!("invalid at: {raw}")))?,
    None => chrono::Local::now().naive_local(),
  };

  let rider = Rider { payment: query.payment.unwrap_or_default(), category: query.category.unwrap_or_default() };
  let itineraries = state.planner.plan(Trip { from, to, at, rider, mode: query.mode.unwrap_or_default() }).await?;

  tracing::info!(from = query.from, to = query.to, %at, options = itineraries.len(), "GET 200 /plan");
  Ok(Json(itineraries))
}
