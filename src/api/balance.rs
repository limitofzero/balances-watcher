use std::sync::Arc;
use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Serialize;
use crate::app_state::AppState;
use alloy::{primitives::Address};
use alloy::primitives::U256;
use alloy::providers::ProviderBuilder;
use axum::http::StatusCode;
use crate::evm::erc20::ERC20;
use crate::evm::networks::EvmNetworks;

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: String,
}


pub async fn get_token_balance(Path((chain, owner, token)): Path<(EvmNetworks, Address, Address)>, State(state): State<Arc<AppState>>) -> Result<Json<BalanceResponse>, (StatusCode, String)> {
    let provider = match state.providers.get(&chain) {
        Some(provider) => provider,
        None => return Err((StatusCode::NOT_FOUND, "Unsupported network".to_string()))
    };

    let erc20 = ERC20::new(token, provider);
    let balance = match erc20.balanceOf(owner).call().await {
        Ok(balance) => balance,
        Err(e) => {
            println!("Error getting balance: {}", e);
            U256::from(0)
        }
    };

    Ok(Json(BalanceResponse{ balance: balance.to_string() }))
}