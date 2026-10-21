use super::*;

use entities::place::Place;

#[derive(serde::Deserialize)]
pub struct SearchQuery {
  q: String,
  near: Option<String>,
}

///# GET <MOVA>/places/search?q=18 de julio y ejido&near=-34.9,-56.16
pub async fn search(State(state): State<crate::State>, Query(query): Query<SearchQuery>) -> crate::Res<Vec<Place>> {
  let term = query.q.trim();
  if term.is_empty() {
    return Err(crate::Error::BadRequest("q must not be empty".into()));
  }
  let near = query.near.as_deref().map(point).transpose()?;

  let hits = Place::search(&state.transit, term, near).await?;
  tracing::info!(term, hits = hits.len(), "GET 200 /places/search");
  Ok(Json(hits))
}

///# GET <MOVA>/place/:id
pub async fn place(State(state): State<crate::State>, Path(id): Path<String>) -> crate::Res<Place> {
  let place = Place::load(&state.transit, &id).await?;
  tracing::info!(id, "GET 200 /place/:id");
  Ok(Json(place))
}

#[derive(serde::Deserialize)]
pub struct ReverseQuery {
  at: String,
}

///# GET <MOVA>/places/reverse?at=-34.9,-56.16
pub async fn reverse(
  State(state): State<crate::State>,
  Query(query): Query<ReverseQuery>,
) -> crate::Res<Option<Place>> {
  let at = point(&query.at)?;
  let place = Place::reverse(&state.transit, at).await?;
  tracing::info!(at = query.at, found = place.is_some(), "GET 200 /places/reverse");
  Ok(Json(place))
}
