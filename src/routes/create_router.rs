use std::sync::Arc;
use axum::{Router, routing::get};
use crate::app_state::AppState;
use crate::api::tokens_list::get_token_list;
use crate::api::balance::get_token_balance;

pub  fn create_router(app_state: Arc<AppState>) -> Router {
    return Router::new()
        .route("/{chain_id}/tokens-list", get(get_token_list))
        .route("/{chain_id}/balance/{owner}/{token}", get(get_token_balance))
        .with_state(app_state);
}