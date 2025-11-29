use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{primitives::Address};
#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}


pub async fn get_balances(Path((network, owner)): Path<(EvmNetworks, Address)>, State(state): State<Arc<AppState>>) -> Json<BalancesResponse> {
    let provider = state.providers.get(&network);
    
    
    
    Json(BalancesResponse{ balances: HashMap::new() })
}