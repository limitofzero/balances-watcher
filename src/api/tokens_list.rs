use crate::app_state::AppState;
use crate::config::network_config::TokenList;
use crate::domain::{EvmNetworks, Token};
use crate::services::tokens_from_list;
use alloy::primitives::Address;
use axum::{extract::Path, extract::State, Json};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn get_token_list(
    Path(network): Path<EvmNetworks>,
    State(state): State<Arc<AppState>>,
) -> Json<HashMap<Address, Token>> {
    let default_list: Vec<TokenList> = vec![];
    let network_token_list = state
        .network_config
        .token_list(network)
        .unwrap_or(&default_list);

    let active_tokens = tokens_from_list::get_tokens_from_list(&network_token_list, network).await;

    Json(active_tokens)
}
