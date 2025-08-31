use axum::{routing::get, Router};
use crate::state::AppState;
use super::handlers::{index, health};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/", get(index))
        .with_state(state)
}
