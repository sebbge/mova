use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

pub mod bus;
pub mod health;
pub mod line;
pub mod routes;
pub mod stop;

use crate::transit::entities;
