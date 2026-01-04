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
    services::tokens_from_list::get_tokens_from_list,
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
    if body.custom_tokens.len() == 0 && body.tokens_lists_urls.len() == 0 {
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

    let mut tokens = get_tokens_from_list(&body.tokens_lists_urls, network)
        .await
        .map_err(|err| AppError::BadRequest(err.to_string()))?;
    tokens.extend(body.custom_tokens);

    let mut watched_tokens = sub.tokens.write().await;
    let prev_count = watched_tokens.len();
    watched_tokens.extend(tokens);
    let new_count = watched_tokens.len();

    tracing::info!(
        "tokens were updated, prev count: {}, new count: {}",
        prev_count,
        new_count
    );

    Ok(())
}
