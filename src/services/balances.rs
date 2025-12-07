use crate::config::network_config::TokenList;
use crate::evm::networks;
use crate::services::{tokens_from_list, errors::ServiceError};
use std::collections::HashMap;
use alloy::primitives::Address;
use alloy::providers::{DynProvider, Provider};
use crate::evm::erc20::ERC20;



pub async fn get_balances(token_list: &Vec<TokenList>, provider: &DynProvider, network: networks::EvmNetworks, owner: Address) -> Result<HashMap<Address, String>, ServiceError> {
    let active_tokens = tokens_from_list::get_tokens_from_list(token_list, network).await;

    let tokens: Vec<Address> = active_tokens.keys().cloned().collect();

    let mut balances_mc  = provider.multicall().dynamic();
    for address in &tokens {
        let contract = ERC20::new(address.clone(), provider);
        let balance_of = contract.balanceOf(owner);
        balances_mc = balances_mc.add_dynamic(balance_of)
    }

    let balances_resp = match balances_mc.try_aggregate(false).await {
        Ok(b) => b,
        Err(e) => return Err(ServiceError::BalancesMultiCallError(e.to_string())),
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

    Ok(balances)
}