use std::sync::Arc;
use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};
use crate::app_state::AppState;
use crate::api::tokens_list::get_token_list;
use crate::api::balance::get_token_balance;
use crate::api::balances::get_balances;

pub  fn create_router(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/{chain_id}/tokens-list", get(get_token_list))
        .route("/sse/{chain_id}/balances/{owner}", get(get_balances))
        .route("/{chain_id}/balance/{owner}/{token}", get(get_token_balance))
        .layer(cors)
        .with_state(app_state)
}