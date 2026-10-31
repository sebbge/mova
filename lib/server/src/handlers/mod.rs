use axum::Json;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::StatusCode;

pub mod bus;
pub mod health;
pub mod place;
pub mod plan;
pub mod routes;
pub mod stop;

use crate::transit::{entities, scope::Scope};

/// `{provider}/{id}` path, provider resolved
pub struct Scoped {
  pub scope: Scope,
  pub provider: String,
  pub id: String,
}

impl FromRequestParts<crate::State> for Scoped {
  type Rejection = crate::Error;

  async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &crate::State) -> crate::Result<Self> {
    let Path((provider, id)) = Path::<(String, String)>::from_request_parts(parts, state)
      .await
      .map_err(|e| crate::Error::BadRequest(e.body_text()))?;
    let scope = state.transit.resolve(&provider)?;
    Ok(Self { scope, provider, id })
  }
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
  q: String,
  near: Option<String>,
}

impl SearchQuery {
  fn term(&self) -> crate::Result<&str> {
    let term = self.q.trim();
    if term.is_empty() {
      return Err(crate::Error::BadRequest("q must not be empty".into()));
    }
    Ok(term)
  }

  fn near(&self) -> crate::Result<Option<crate::sdk::geo::Point>> {
    self.near.as_deref().map(point).transpose()
  }
}

pub fn point(raw: &str) -> crate::Result<crate::sdk::geo::Point> {
  let invalid = || crate::Error::BadRequest(format!("invalid point: {raw}"));
  let (lat, lng) = raw.split_once(',').ok_or_else(invalid)?;
  let (lat, lng): (f64, f64) = (lat.trim().parse().map_err(|_| invalid())?, lng.trim().parse().map_err(|_| invalid())?);
  if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
    return Err(invalid());
  }
  Ok(crate::sdk::geo::Point { lat, lng })
}
