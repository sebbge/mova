use super::*;

use entities::line::{Route, RouteShape};

///# GET <BONDI>/routes
pub async fn list(State(state): State<crate::State>) -> crate::Res<Vec<Route>> {
  let routes = Route::load_list(&state).await?;

  tracing::info!(count = routes.len(), "GET 200 /routes");
  Ok(Json(routes))
}

///# GET <BONDI>/routes/search?q=<term>
pub async fn search(State(state): State<crate::State>, Query(params): Query<SearchQuery>) -> crate::Res<Vec<Route>> {
  if params.q.is_empty() {
    return Err(crate::Error::BadRequest("q must not be empty".into()));
  }

  let routes = Route::load_search(&state, &params.q).await?;
  tracing::info!(term = params.q, "GET 200 /routes/search");
  Ok(Json(routes))
}

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
  pub q: String,
}

///# GET <BONDI>/route/shape/:provider/:id?direction_id=0
pub async fn shape(
  State(state): State<crate::State>,
  Path((provider, route_id)): Path<(String, String)>,
  Query(params): Query<ShapeQuery>,
) -> crate::Res<RouteShape> {
  let scope = state.resolve(&provider)?;
  let direction = params.direction_id.unwrap_or(0);

  let shape = RouteShape::load(&state, &scope, &route_id, direction).await?;
  tracing::info!(provider, route_id, dir = direction, "GET 200 /route/shape/:provider/:id");
  Ok(Json(shape))
}

///# GET <BONDI>/shape/:provider/:id
pub async fn variant(
  State(state): State<crate::State>,
  Path((provider, shape_id)): Path<(String, String)>,
) -> crate::Res<RouteShape> {
  let scope = state.resolve(&provider)?;
  let shape = RouteShape::load_by_shape(&state, &scope, &shape_id).await?;
  tracing::info!(provider, shape_id, "GET 200 /shape/:provider/:id");
  Ok(Json(shape))
}

#[derive(Debug, serde::Deserialize)]
pub struct ShapeQuery {
  pub direction_id: Option<i16>,
}
