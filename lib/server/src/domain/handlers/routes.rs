use super::*;

use entities::line::{Route, RouteShape};

///# GET <BONDI>/routes
pub async fn list(State(state): State<crate::State>) -> crate::Res<Vec<Route>> {
  if let Some(cached) = Route::cache_get_list(&state.cache).await {
    return Ok(Json(cached));
  }

  let routes = Route::list(&state.db).await?;
  Route::cache_set_list(&state.cache, &routes).await;

  tracing::info!(count = routes.len(), "GET 200 /routes");
  Ok(Json(routes))
}

///# GET <BONDI>/routes/search?q=<term>
pub async fn search(State(state): State<crate::State>, Query(params): Query<SearchQuery>) -> crate::Res<Vec<Route>> {
  if params.q.is_empty() {
    return Err(crate::Error::BadRequest("q must not be empty".into()));
  }

  if let Some(cached) = Route::cache_get_search(&state.cache, &params.q).await {
    return Ok(Json(cached));
  }

  let routes = Route::search(&state.db, &params.q).await?;
  Route::cache_set_search(&state.cache, &params.q, &routes).await;
  tracing::info!(term = params.q, "GET 200 /routes/search");
  Ok(Json(routes))
}

#[derive(Debug, serde::Deserialize)]
pub struct SearchQuery {
  pub q: String,
}

///# GET <BONDI>/routes/:route_id/shape?direction_id=0
pub async fn shape(
  State(state): State<crate::State>,
  Path(route_id): Path<String>,
  Query(params): Query<ShapeQuery>,
) -> crate::Res<RouteShape> {
  let direction = params.direction_id.unwrap_or(0);

  if let Some(cached) = RouteShape::cache_get(&state.cache, &route_id, direction).await {
    return Ok(Json(cached));
  }

  let shape = RouteShape::get(&state.db, &route_id, direction).await?;
  RouteShape::cache_set(&state.cache, &shape.route_id, direction, &shape).await;
  tracing::info!(route_id = route_id, dir = direction, "GET 200 /routes/:route_id/shape");
  Ok(Json(shape))
}

#[derive(Debug, serde::Deserialize)]
pub struct ShapeQuery {
  pub direction_id: Option<i16>,
}
