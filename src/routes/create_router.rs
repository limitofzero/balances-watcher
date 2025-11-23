use std::sync::Arc;
use axum::{Router, routing::get};
use crate::app_state::AppState;
use crate::handlers::tokens_list::get_token_list;

pub  fn create_router(app_state: Arc<AppState>) -> Router {
    return Router::new()
        .route("/{id}/tokens-list", get(get_token_list))
        .with_state(app_state);
}