use axum::{Router, routing::get};

use crate::{api::routes::get_search, state::AppState};

pub mod dto;
mod routes;

pub fn get_api_router() -> Router<AppState> {
    Router::new().route("/search", get(get_search))
}
