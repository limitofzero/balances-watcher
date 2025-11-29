use std::sync::Arc;
use axum::{Router, routing::get};
use crate::app_state::AppState;
use crate::api::tokens_list::get_token_list;
use crate::api::balance::get_token_balance;
use crate::api::balances::get_balances;

pub  fn create_router(app_state: Arc<AppState>) -> Router {
    return Router::new()
        .route("/{chain_id}/tokens-list", get(get_token_list))
        .route("/{chain_id}/balances/{owner}", get(get_balances))
        .route("/{chain_id}/balance/{owner}/{token}", get(get_token_balance))
        .with_state(app_state);
}