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
use crate::services::tokens_from_list;

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

    let tokens = tokens_from_list::get_tokens_from_list(&network_token_list, network).await;
    let tokens: Vec<Address> = tokens.keys().cloned().collect();

    let mut balances_mc  = provider.multicall().dynamic();
    for address in &tokens {
        let contract = ERC20::new(address.clone(), provider);
        let balance_of = contract.balanceOf(owner);
        balances_mc = balances_mc.add_dynamic(balance_of)
    }

    let balances_resp = match balances_mc.try_aggregate(false).await {
        Ok(b) => b,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    };

    let mut balances: HashMap<Address, String> = HashMap::new();
    for (i, balance) in balances_resp.iter().enumerate() {
        match balance {
            Ok(correct_balance) => { balances.insert(tokens[i].clone(), correct_balance.to_string()); }
            Err(_) => {
                println!("Error getting balance for token {}", tokens[i]);
            }
        }
    }

    Ok(Json(BalancesResponse{ balances }))
}