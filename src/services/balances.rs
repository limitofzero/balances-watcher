use crate::domain::EvmNetwork;
use crate::evm::{erc20::ERC20, multicall3::Multicall3};
use crate::services::errors::ServiceError;
use alloy::eips::BlockId;
use alloy::primitives::{Address, U256};
use alloy::providers::DynProvider;
use alloy::sol_types::{SolCall, SolValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub struct BalanceCallCtx {
    pub network: EvmNetwork,
    pub owner: Address,
    pub provider: Arc<DynProvider>,
    pub multicall3: Address,
}

pub async fn get_balances(
    ctx: Arc<BalanceCallCtx>,
    tokens: &[Address],
    block_id: Option<BlockId>,
) -> Result<HashMap<Address, U256>, ServiceError> {
    let native_address = ctx.network.native_token_address();
    let mut erc20_tokens: Vec<Address> = tokens
        .iter()
        .cloned()
        .filter(|a| *a != native_address)
        .collect();
    erc20_tokens.sort();

    // todo check that clone is not expensive here
    let multicall3 = Multicall3::new(ctx.multicall3, ctx.provider.clone());
    // one for erc balances
    let mut calls: Vec<Multicall3::Call3> = Vec::with_capacity(erc20_tokens.len() + 1);
    let owner = ctx.owner;

    for address in &erc20_tokens {
        let call = ERC20::balanceOfCall { owner };
        let calldata = call.abi_encode();
        calls.push(Multicall3::Call3 {
            target: *address,
            allowFailure: true,
            callData: calldata.into(),
        });
    }

    let eth_balance_call = Multicall3::getEthBalanceCall { addr: ctx.owner };
    let eth_balance_call_data = eth_balance_call.abi_encode();
    calls.push(Multicall3::Call3 {
        target: ctx.multicall3,
        allowFailure: true,
        callData: eth_balance_call_data.into(),
    });

    let block_id = block_id.unwrap_or(BlockId::latest());
    let t0 = Instant::now();
    let balances_resp = multicall3
        .aggregate3(calls)
        .block(block_id)
        .call()
        .await
        .map_err(|e| ServiceError::BalancesMultiCallError(e.to_string()))?;

    tracing::info!(
        time = t0.elapsed().as_millis(),
        "aggregate3 balances complete"
    );

    let mut balances: HashMap<Address, U256> = HashMap::with_capacity(erc20_tokens.len() + 1);

    for (i, erc20_token) in erc20_tokens.iter().enumerate() {
        let resp = &balances_resp.get(i).ok_or_else(|| {
            ServiceError::BalancesMultiCallError(
                "multicall3: missing response at index={i} for token={token}".to_string(),
            )
        })?;

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
                balances.insert(*erc20_token, balance);
            }
            Err(e) => {
                tracing::error!(error = %e, "abi_decode failed for {}", erc20_token);
            }
        }
    }

    let eth_balance_resp = balances_resp.get(erc20_tokens.len()).ok_or_else(|| {
        ServiceError::BalancesMultiCallError(
            "multicall3: missing response at index={i} for token={token}".to_string(),
        )
    })?;

    match <U256 as SolValue>::abi_decode(&eth_balance_resp.returnData) {
        Ok(balance) => {
            balances.insert(native_address, balance);
        }
        Err(e) => {
            tracing::error!(error = %e, "abi_decode failed for {}", native_address);
        }
    }

    Ok(balances)
}
