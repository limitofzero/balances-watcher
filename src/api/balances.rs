use std::{convert::Infallible, collections::HashMap, time::Duration, sync::Arc};
use axum::{response::sse::{Sse, Event}, extract::{Path, State}, http::StatusCode};
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{ primitives::Address};
use alloy::primitives::U256;
use alloy::providers::{DynProvider, Provider};
use alloy::rpc::types::{Filter, Log, Topic};
use alloy::sol_types::SolEvent;
use crate::config::network_config::TokenList;
use crate::services::{balances, tokens_from_list};
use futures::{Stream, StreamExt};
use tokio::time::interval;
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};
use crate::evm::erc20::ERC20;
use crate::evm::token::Token;

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

struct BalanceContext {
    owner: Address,
    provider: DynProvider,
    tokens: HashMap<Address, Token>,
    network: EvmNetworks,
}

#[derive(Serialize)]
struct BalanceStreamError {
    error: String,
}

#[derive(Serialize)]
struct TokenBalance {
    address: Address,
    balance: U256,
}

pub async fn get_balances(
    Path((network, owner)): Path<(EvmNetworks, Address)>,
    State(state): State<Arc<AppState>>
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider.clone(),
        None => return Err(StatusCode::NOT_FOUND),
    };

    let ws_provider = match state.ws_providers.get(&network) {
        None => {
            return Err(StatusCode::NOT_FOUND);
        }
        Some(ws_provider) => ws_provider.clone(),
    };

    let network_token_list: Vec<TokenList> = state
        .network_config
        .token_list(network)
        .cloned()
        .unwrap_or_default();

    let tokens = tokens_from_list::get_tokens_from_list(&network_token_list, network).await;

    let ctx = Arc::new(BalanceContext {
        provider,
        tokens,
        owner,
        network,
    });

    let interval = interval(Duration::from_secs(60));

    let base = Filter::new().event_signature(ERC20::Transfer::SIGNATURE_HASH);
    let from_filter = base.clone().topic1(Topic::from(owner));
    let to_filter = base.clone().topic2(Topic::from(owner));

    let mut from_subscribe = ws_provider
        .subscribe_logs(&from_filter)
        .await
        .or_else(|_| Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut to_subscribe = ws_provider
        .subscribe_logs(&to_filter)
        .await
        .or_else(|_| Err(StatusCode::INTERNAL_SERVER_ERROR))?;

    let ctx_for_from = Arc::clone(&ctx);
    let from_stream = from_subscribe
        .into_stream()
        .filter_map(move |log| {
            let ctx = Arc::clone(&ctx_for_from);
            async move {
                parse_transfer_and_get_balance(ctx, &log).await
            }
    });

    let ctx_for_to = Arc::clone(&ctx);
    let to_stream = to_subscribe
        .into_stream()
        .filter_map(move |log| {
            let ctx = Arc::clone(&ctx_for_to);
            async move { parse_transfer_and_get_balance(ctx, &log).await }
        });

    let multicall_interval_handle = IntervalStream::new(interval)
        .then(move |_| {
            let ctx = Arc::clone(&ctx);
            
            async move {
                let result = balances::get_balances(
                    &ctx.tokens,
                    &ctx.provider,
                    ctx.owner,
                    ctx.network
                ).await;

                let event = match result {
                    Ok(balances) =>
                        Event::default()
                            .event("balances")
                            .json_data(BalancesResponse { balances })
                            .unwrap()
                    ,
                    Err(e) =>
                        Event::default()
                            .event("error")
                            .json_data(BalanceStreamError { error: e.to_string() })
                            .unwrap()

                };

                Ok::<Event, Infallible>(event)
            }
        });

    let (tx, rx) = tokio::sync::mpsc::channel::<Event>(256);

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let s = from_stream;
            futures::pin_mut!(s);
            while let Some (result) = s.next().await {
                let event = Event::default()
                    .event("update_balance")
                    .json_data(&result)
                    .unwrap();

                if  tx.send(event).await.is_err() {
                    break;
                }
            }
        });
    }

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let s = to_stream;
            futures::pin_mut!(s);
            while let Some (result) = s.next().await {
                let event = Event::default()
                    .event("update_balance")
                    .json_data(&result)
                    .unwrap();

                if  tx.send(event).await.is_err() {
                    break;
                }
            }
        });
    }

    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let s = multicall_interval_handle;
            futures::pin_mut!(s);
            while let Some(result) = s.next().await {
                if let Ok(event) = result {
                    if tx.send(event).await.is_err() {
                        break;
                    }
                }
            }
        });
    }

    drop(tx);
    let sse_stream = ReceiverStream::new(rx).map(Ok::<Event, Infallible>);

    Ok(Sse::new(sse_stream))
}



async fn parse_transfer_and_get_balance(ctx: Arc<BalanceContext>, log: &Log) -> Option<TokenBalance> {
    let log: Log<ERC20::Transfer> = match log.log_decode() {
        Ok(log) => log,
        Err(_) => return None,
    };

    let erc20 = ERC20::new(log.address(), &ctx.provider);
    match erc20.balanceOf(ctx.owner).call().await {
        Ok(balance) => Some(TokenBalance {
            address: log.address(),
            balance,
        }),
        Err(e) => {
            tracing::error!("failed to get balance for {}: {:?}", log.address(), e);
            None
        },
    }
}