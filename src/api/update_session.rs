use std::sync::Arc;

use alloy::primitives::Address;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app_error::AppError,
    app_state::AppState,
    domain::{EvmNetwork, SubscriptionKey},
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionRequest {
    #[serde(default)]
    tokens_lists_urls: Vec<String>,

    #[serde(default)]
    custom_tokens: Vec<Address>,
}

pub async fn update_session(
    Path((network, owner)): Path<(EvmNetwork, Address)>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateSessionRequest>,
) -> Result<(), AppError> {
    if body.custom_tokens.is_empty() && body.tokens_lists_urls.is_empty() {
        return Err(AppError::BadRequest(
            "tokens_lists_urls && custom_tokens are empty".to_string(),
        ));
    }

    let key = SubscriptionKey { network, owner };

    let sub = state
        .sub_manager
        .get_subscription(key)
        .await
        .ok_or(AppError::NoSession(network, owner))?;

    let token_list_fetcher = Arc::clone(&state.token_list_fetcher);

    let mut tokens = token_list_fetcher
        .get_tokens(&body.tokens_lists_urls, network)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;
    tokens.extend(body.custom_tokens);

    let mut watched_tokens = sub.tokens.write().await;
    let prev_count = watched_tokens.len();

    // count how many new unique tokens would be added
    let new_unique = tokens
        .iter()
        .filter(|t| !watched_tokens.contains(*t))
        .count();

    if prev_count + new_unique > state.network_config.max_watched_tokens_limit {
        tracing::error!(
            "limit of watched tokens was exceeded: {}",
            prev_count + new_unique
        );
        return Err(AppError::TokenLimitExceeded);
    }

    watched_tokens.extend(tokens);
    let new_count = watched_tokens.len();

    tracing::info!(
        "watched token list was updated, prev watched token count: {}, new count: {}",
        prev_count,
        new_count
    );

    Ok(())
}
