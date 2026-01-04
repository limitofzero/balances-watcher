use crate::app_error::AppError;
use crate::app_state::AppState;
use crate::domain::EvmNetwork;
use crate::evm::erc20::ERC20;
use alloy::primitives::Address;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct BalanceResponse {
    pub balance: String,
}

pub async fn get_token_balance(
    Path((chain, owner, token)): Path<(EvmNetwork, Address, Address)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<BalanceResponse>, AppError> {
    let provider = state
        .providers
        .get(&chain)
        .ok_or(AppError::ProviderIsNotDefined(chain))?;

    let erc20 = ERC20::new(token, provider);
    let balance = erc20
        .balanceOf(owner)
        .call()
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(Json(BalanceResponse {
        balance: balance.to_string(),
    }))
}
