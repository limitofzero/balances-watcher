use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{primitives::Address};
use alloy::providers::Provider;
use alloy::transports::http::reqwest::StatusCode;
use crate::config::network_config::TokenList;
use crate::evm::{errors::EvmError, multicall, erc20::ERC20, erc20};
use crate::services::balances;

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

pub async fn get_balances(Path((network, owner)): Path<(EvmNetworks, Address)>, State(state): State<Arc<AppState>>) -> Result<Json<BalancesResponse>, (StatusCode, String)> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider,
        None => return Err((StatusCode::NOT_FOUND, EvmError::UnsupportedNetwork(network.chain_id()).to_string())),
    };

    let default_list: Vec<TokenList> = vec![];
    let network_token_list = state
        .network_config
        .token_list(network)
        .unwrap_or(&default_list);

    match balances::get_balances(&network_token_list, provider, network, owner).await {
        Ok(balances) => Ok(Json(BalancesResponse{ balances })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}