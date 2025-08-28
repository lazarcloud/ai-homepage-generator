use axum::{routing::get, Router};
use crate::state::AppState;
use super::handlers::index;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .with_state(state)
}
