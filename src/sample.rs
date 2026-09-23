// /sample

use crate::state::AppState;
use axum::Router;
use std::sync::Arc;

async fn index_get() -> String {
    "pos-down-detector".to_string()
}

pub fn api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", axum::routing::get(index_get))
        .with_state(state)
}
