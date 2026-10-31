use super::*;

use entities::line::{DayType, Route, RouteSchedule, RouteShape, RouteTimes};

///# GET <MOVA>/routes
pub async fn list(State(state): State<crate::State>) -> crate::Res<Vec<Route>> {
  let routes = Route::load_list(&state.transit).await?;

  tracing::info!(count = routes.len(), "GET 200 /routes");
  Ok(Json(routes))
}

///# GET <MOVA>/routes/search?q=<term>
pub async fn search(State(state): State<crate::State>, Query(query): Query<SearchQuery>) -> crate::Res<Vec<Route>> {
  let term = query.term()?;
  let routes = Route::load_search(&state.transit, term).await?;
  tracing::info!(term, "GET 200 /routes/search");
  Ok(Json(routes))
}

///# GET <MOVA>/route/shape/:provider/:id?direction_id=0
pub async fn shape(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: route_id }: Scoped,
  Query(params): Query<ShapeQuery>,
) -> crate::Res<RouteShape> {
  let direction = params.direction_id.unwrap_or(0);

  let shape = RouteShape::load(&state.transit, &scope, &route_id, direction).await?;
  tracing::info!(provider, route_id, dir = direction, "GET 200 /route/shape/:provider/:id");
  Ok(Json(shape))
}

///# GET <MOVA>/shape/:provider/:id
pub async fn variant(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: shape_id }: Scoped,
) -> crate::Res<RouteShape> {
  let shape = RouteShape::load_by_shape(&state.transit, &scope, &shape_id).await?;
  tracing::info!(provider, shape_id, "GET 200 /shape/:provider/:id");
  Ok(Json(shape))
}

///# GET <MOVA>/route/times/:provider/:id?direction_id=0&shape_id=<id>&stop=<id>
pub async fn times(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: route_id }: Scoped,
  Query(params): Query<TimesQuery>,
) -> crate::Res<RouteTimes> {
  let direction = params.direction_id.unwrap_or(0);
  let (shape_id, stop) = (params.shape_id.as_deref(), params.stop.as_deref());

  let times = RouteTimes::load(&state.transit, &scope, &route_id, direction, shape_id, stop).await?;
  tracing::info!(provider, route_id, dir = direction, stop, "GET 200 /route/times/:provider/:id");
  Ok(Json(times))
}

///# GET <MOVA>/route/schedule/:provider/:id?direction_id=0&stop=<id>&day=habil|sabado|domingo
pub async fn schedule(
  State(state): State<crate::State>,
  Scoped { scope, provider, id: route_id }: Scoped,
  Query(params): Query<ScheduleQuery>,
) -> crate::Res<RouteSchedule> {
  let direction = params.direction_id.unwrap_or(0);

  let schedule = RouteSchedule::load(&state.transit, &scope, &route_id, direction, &params.stop, params.day).await?;
  tracing::info!(provider, route_id, dir = direction, stop = params.stop, "GET 200 /route/schedule/:provider/:id");
  Ok(Json(schedule))
}

#[derive(Debug, serde::Deserialize)]
pub struct ScheduleQuery {
  pub direction_id: Option<i16>,
  pub stop: String,
  /// Defaults to today's day type
  pub day: Option<DayType>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ShapeQuery {
  pub direction_id: Option<i16>,
}

#[derive(Debug, serde::Deserialize)]
pub struct TimesQuery {
  pub direction_id: Option<i16>,
  /// Shown shape, defaults to the direction's main one
  pub shape_id: Option<String>,
  /// Defaults to the first stop
  pub stop: Option<String>,
}
