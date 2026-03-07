use std::sync::Arc;

use alloy::primitives::Address;
use axum::{
    extract::{Path, State},
    Json,
};
use metrics::counter;
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

    let total_unique = prev_count + new_unique;
    if total_unique > state.network_config.max_watched_tokens_limit {
        counter!("tokens_limit_exceeded_total").increment(1);
        tracing::error!(
            tokens_len = total_unique,
            previous_tokens_len = prev_count,
            "limit of watched tokens was exceeded",
        );
        return Err(AppError::TokenLimitExceeded);
    }

    watched_tokens.extend(tokens);
    let new_count = watched_tokens.len();

    tracing::info!(
        tokens_len_before = prev_count,
        current_tokens_len = new_count,
        sub = %key,
        "session was updated",
    );

    Ok(())
}
