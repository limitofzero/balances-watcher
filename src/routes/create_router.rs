use crate::api::balances::get_balances;
use crate::api::update_session::update_session;
use crate::api::{balance::get_token_balance, create_session::create_session};
use crate::app_state::AppState;
use axum::routing::put;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/sse/{chain_id}/balances/{owner}", get(get_balances))
        .route("/{chain_id}/sessions/{owner}", post(create_session))
        .route("/{chain_id}/sessions/{owner}", put(update_session))
        .route(
            "/{chain_id}/balance/{owner}/{token}",
            get(get_token_balance),
        )
        .layer(cors)
        .with_state(app_state)
}
