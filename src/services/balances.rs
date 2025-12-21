use crate::services::{errors::ServiceError};
use std::collections::HashMap;
use std::time::Instant;
use alloy::primitives::Address;
use alloy::providers::{DynProvider, Provider};
use crate::evm::erc20::ERC20;
use crate::evm::networks::EvmNetworks;
use crate::evm::token::Token;

pub async fn get_balances(tokens: &HashMap<Address, Token>, provider: &DynProvider, owner: Address, network: EvmNetworks) -> Result<HashMap<Address, String>, ServiceError> {
    let native_address = network.native_token_address();
    let erc20_tokens: Vec<Address> = tokens.keys().cloned().filter(|a| *a != native_address).collect();

    let mut balances_mc = provider.multicall().dynamic();

    for address in &erc20_tokens {
        let contract = ERC20::new(address.clone(), provider);
        let balance_of = contract.balanceOf(owner);
        balances_mc = balances_mc.add_dynamic(balance_of);
    }

    let t0 = Instant::now();
    let balances_resp = match balances_mc.try_aggregate(false).await {
        Ok(b) => b,
        Err(e) => return Err(ServiceError::BalancesMultiCallError(e.to_string())),
    };

    tracing::info!(time = t0.elapsed().as_secs(), "aggregate balances complete");

    let mut balances: HashMap<Address, String> = HashMap::new();
    for (i, balance) in balances_resp.iter().enumerate() {
        match balance {
            Ok(correct_balance) => { balances.insert(erc20_tokens[i].clone(), correct_balance.to_string()); }
            Err(_) => {
                tracing::warn!("Error getting balance for token {}", erc20_tokens[i]);
            }
        }
    }

    Ok(balances)
}