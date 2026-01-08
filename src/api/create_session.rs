use std::sync::Arc;

use alloy::primitives::Address;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::{
    app_error::AppError, app_state::AppState, config::constants::DEFAULT_MAX_WATCHED_TOKENS_LIMIT, domain::{EvmNetwork, SubscriptionKey}
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    tokens_lists_urls: Vec<String>,

    #[serde(default)]
    custom_tokens: Vec<Address>,
}

pub async fn create_session(
    Path((network, owner)): Path<(EvmNetwork, Address)>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateSessionRequest>,
) -> Result<(), AppError> {
    if body.tokens_lists_urls.is_empty() {
        return Err(AppError::BadRequest("tokens_lists_urls should not be empty".into()));
    }

    let key = SubscriptionKey { network, owner };

    let fetcher = Arc::clone(&state.token_list_fetcher);

    let mut tokens = fetcher.get_tokens(&body.tokens_lists_urls, network)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;

    let mut combined = tokens.clone();
    combined.extend(body.custom_tokens.clone());

    if combined.len() > DEFAULT_MAX_WATCHED_TOKENS_LIMIT {
        return Err(AppError::TokenLimitExceeded);
    }

    tokens.extend(body.custom_tokens);

    let _ = state.sub_manager.create_or_update(key, tokens).await;

    tracing::warn!(
        "session for wallet:network {}:{} was created, watched tokens count is {}",
        owner,
        network,
        combined.len(),
    );

    Ok(())
}
