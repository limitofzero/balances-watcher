use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{primitives::Address};
use alloy::transports::http::reqwest::StatusCode;
use crate::evm::{errors::EvmError, multicall};

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}


pub async fn get_balances(Path((network, owner)): Path<(EvmNetworks, Address)>, State(state): State<Arc<AppState>>) -> Result<Json<BalancesResponse>, (StatusCode, String)> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider,
        None => return Err((StatusCode::NOT_FOUND, EvmError::UnsupportedNetwork(network.chain_id()).to_string())),
    };

    let multicall_addr = state.network_config.multicall_address;
    if multicall_addr.is_zero() {
        return Err((StatusCode::NOT_FOUND, "Multicall address is not set".to_string()));
    }

    let multicall = multicall::Multicall::new(multicall_addr, provider);


    Ok(Json(BalancesResponse{ balances: HashMap::new() }))
}