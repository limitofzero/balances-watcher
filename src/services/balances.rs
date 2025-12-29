use crate::services::{errors::ServiceError};
use std::collections::HashMap;
use std::time::Instant;
use alloy::primitives::{Address, U256};
use alloy::providers::{DynProvider};
use alloy::sol_types::{SolCall, SolValue};
use crate::evm::{erc20::ERC20, multicall3::Multicall3};
use crate::evm::networks::EvmNetworks;
use crate::evm::token::Token;

pub async fn get_balances(tokens: &HashMap<Address, Token>, provider: &DynProvider, owner: Address, network: EvmNetworks, multicall3_add: Address) -> Result<HashMap<Address, String>, ServiceError> {
    let native_address = network.native_token_address();
    let mut erc20_tokens: Vec<Address> = tokens.keys().cloned().filter(|a| *a != native_address).collect();
    erc20_tokens.sort();

    let multicall3 = Multicall3::new(multicall3_add, provider);
    // one for erc balances
    let mut calls: Vec<Multicall3::Call3> = Vec::with_capacity(erc20_tokens.len() + 1);

    for address in &erc20_tokens {
        let call = ERC20::balanceOfCall { owner };
        let calldata = call.abi_encode();
        calls.push(Multicall3::Call3{
            target: *address,
            allowFailure: true,
            callData: calldata.into(),
        });
    }

    let eth_balance_call = Multicall3::getEthBalanceCall{
        addr: owner,
    };
    let eth_balance_call_data = eth_balance_call.abi_encode();
    calls.push(Multicall3::Call3{
        target: multicall3_add,
        allowFailure: true,
        callData: eth_balance_call_data.into(),
    });

    let t0 = Instant::now();
    let balances_resp = multicall3
        .aggregate3(calls)
        .call()
        .await
        .map_err(|e| ServiceError::BalancesMultiCallError(e.to_string()))?;

    tracing::info!(time = t0.elapsed().as_secs(), "aggregate3 balances complete");

    let mut balances: HashMap<Address, String> = HashMap::with_capacity(erc20_tokens.len() + 1);

    for (i, erc20_token) in erc20_tokens.iter().enumerate() {
        let resp = &balances_resp
            .get(i)
            .ok_or_else(|| ServiceError::BalancesMultiCallError("multicall3: missing response at index={i} for token={token}".to_string()))?;

        if !resp.success {
            tracing::error!(
                token = %erc20_token,
                index = i,
                return_data_len = resp.returnData.len(),
                "multicall3 subcall failed (success=false)"
            );

            return Err(ServiceError::BalancesMultiCallError(format!(
                "multicall3 subcall failed: token={erc20_token}, index={i}, return_data_len={}",
                resp.returnData.len()
            )));
        }

        match <U256 as SolValue>::abi_decode(&resp.returnData) {
            Ok(balance) => {
                if balance > U256::from(0) {
                    balances.insert(erc20_token.clone(), balance.to_string());
                }
            },
            Err(e) => {
                tracing::error!(error = %e, "abi_decode failed for {}", erc20_token);
            }
        }
    }

    let eth_balance_resp = balances_resp
        .get(erc20_tokens.len())
        .ok_or_else(|| ServiceError::BalancesMultiCallError("multicall3: missing response at index={i} for token={token}".to_string()))?;

    match <U256 as SolValue>::abi_decode(&eth_balance_resp.returnData) {
        Ok(abi_data) => {
            balances.insert(native_address.clone(), abi_data.to_string());
        },
        Err(e) => {
            tracing::error!(error = %e, "abi_decode failed for {}", native_address);
        }
    }


    Ok(balances)
}