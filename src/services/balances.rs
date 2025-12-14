use crate::config::network_config::TokenList;
use crate::evm::networks;
use crate::services::{tokens_from_list, errors::ServiceError};
use std::collections::HashMap;
use std::time::Instant;
use alloy::primitives::Address;
use alloy::providers::{DynProvider, Provider};
use tracing::info;
use crate::evm::erc20::ERC20;
use crate::evm::token::Token;

pub async fn get_balances(tokens: &HashMap<Address, Token>, provider: &DynProvider, owner: Address) -> Result<HashMap<Address, String>, ServiceError> {
    let tokens: Vec<Address> = tokens.keys().cloned().collect();

    let mut balances_mc  = provider.multicall().dynamic();
    for address in &tokens {
        let contract = ERC20::new(address.clone(), provider);
        let balance_of = contract.balanceOf(owner);
        balances_mc = balances_mc.add_dynamic(balance_of)
    }

    let t0 = Instant::now();
    let balances_resp = match balances_mc.try_aggregate(false).await {
        Ok(b) => b,
        Err(e) => return Err(ServiceError::BalancesMultiCallError(e.to_string())),
    };

    info!(time = t0.elapsed().as_secs(), "aggregate balances complete");

    let mut balances: HashMap<Address, String> = HashMap::new();
    for (i, balance) in balances_resp.iter().enumerate() {
        match balance {
            Ok(correct_balance) => { balances.insert(tokens[i].clone(), correct_balance.to_string()); }
            Err(_) => {
                println!("Error getting balance for token {}", tokens[i]);
            }
        }
    }

    Ok(balances)
}