use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use axum::{extract::{Path, State},  response::sse::{Event, Sse}};
use serde::Serialize;
use crate::app_state::{self, AppState};
use crate::evm::networks::EvmNetworks;
use alloy::{primitives::Address, transports::{RpcError, TransportErrorKind}};
use alloy::primitives::U256;
use alloy::providers::{DynProvider, Provider};
use alloy::rpc::types::{Filter, Log, Topic};
use alloy::sol_types::SolEvent;
use crate::config::network_config::TokenList;
use crate::services::{balances, tokens_from_list, subscription_manager, cleanup_stream};
use futures::{Stream, StreamExt};
use tokio::time::{interval};
use tokio_stream::wrappers::{BroadcastStream, IntervalStream, ReceiverStream};
use crate::api::errors::StreamError;
use crate::evm::{erc20::ERC20, token::Token};

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

// todo decompose the struct
struct BalanceContext {
    owner: Address,
    provider: DynProvider,
    tokens: HashMap<Address, Token>,
    network: EvmNetworks,
    multicall3: Address,
    ws_provider: DynProvider,
}


struct TokenBalance {
    address: Address,
    balance: U256,
}

#[derive(Serialize)]
struct TokenBalanceSseEvent {
    address: Address,
    balance: String,
}

#[derive(Serialize)]
struct ErrorBalanceSseEvent {
    code: u16,
    message: String,
}

pub async fn get_balances(
    Path((network, owner)): Path<(EvmNetworks, Address)>,
    State(state): State<Arc<AppState>>
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StreamError> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider.clone(),
        None => return Err(StreamError{
            code: 404,
            message: format!("No provider for network {}", network)
        }),
    };

    let ws_provider = match state.ws_providers.get(&network) {
        None => {
            return Err(StreamError{
                code: 404,
                message: format!("No ws provider for network {}", network)
            });
        }
        Some(ws_provider) => ws_provider.clone(),
    };

    let multicall3 = state.network_config.multicall_address();
    if multicall3.is_empty() {
        return Err(StreamError{
            code: 404,
            message: format!("No multicall3 for network {}", network)
        });
    }

    let network_token_list: Vec<TokenList> = state
        .network_config
        .token_list(network)
        .cloned()
        .unwrap_or_default();

    let tokens = tokens_from_list::get_tokens_from_list(&network_token_list, network).await;

    let sub_key = subscription_manager::SubscriptionKey {
        owner,
        network,
    };

    let (rx, is_first, subscription) = state.sub_manager
        .subscribe(sub_key.clone())
        .await
        .map_err(|e| StreamError{ code: 500, message: e.to_string() })?;

    let ctx = Arc::new(BalanceContext {
        provider,
        tokens,
        owner,
        network,
        multicall3: *multicall3,
        ws_provider,
    });

    let snapshot_interval = state.network_config.snapshot_interval;

    if is_first {
        spawn_balances_snapshot_update(
            Arc::clone(&ctx),
            Arc::clone(&subscription),
            snapshot_interval)
            .await;

        match spawn_from_to_erc20_transfer_updates(Arc::clone(&ctx), Arc::clone(&subscription)).await {
            Ok(()) => {},
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "error when attempt to subscribe to ws erc20 transfer event for {} network {}",
                    ctx.owner,
                    ctx.network,
                );

                let error_event = subscription_manager::BalanceEvent::Error {
                    code: 500,
                    message: "Impossible to subscribe to ws erc20 transfer events".to_string()
                };

                let _ = subscription.sender.send(error_event).inspect_err(|err| {
                    tracing::error!(
                        error = %err,
                        "error when send error event to client for {} network {}",
                        ctx.owner,
                        ctx.network,
                    );
                });
            },
        }
    } else {
        let balance_snapshot = subscription.balances_snapshot.read().await;

        let event = if balance_snapshot.is_empty() {
            subscription_manager::BalanceEvent::Error {
                code: 500,
                message: format!("Empty snapshot for {network} for {owner}")
            }
        } else {
            subscription_manager::BalanceEvent::FullSnapshot(balance_snapshot.clone())
        };

        let _ = subscription.sender.send(event).inspect_err(|err| {
            tracing::error!(
                error = %err,
                "error when send balance_snapshot update for new client {} network {}",
                ctx.owner,
                ctx.network,
            );
        });
    }

    let manager_for_cleanup = Arc::clone(&state.sub_manager);
    let key_for_cleanup = sub_key.clone();

    let sse_stream = BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(event) => {
                    let sse_event = match balance_event_to_sse(event) {
                        Ok(sse_event) => Some(Ok(sse_event)),
                        Err(err) => {
                            tracing::error!(
                                error = %err,
                                "error when convert balance event to sse event",
                            );
                            None
                        }
                    };
                    sse_event
                },
                Err(err) => {
                    tracing::error!(
                        error = %err,
                        "broadcast stream error",
                    );
                    None
                },
            }
        });

    let cleanup_stream = cleanup_stream::CleanupStream::new(sse_stream, manager_for_cleanup, key_for_cleanup);

    Ok(Sse::new(cleanup_stream))
}

fn balance_event_to_sse(event: subscription_manager::BalanceEvent) -> Result<Event, axum::Error> {
    match event {
        subscription_manager::BalanceEvent::FullSnapshot(balances_map) => {
            Event::default()
                .event("all_balances")
                .json_data(BalancesResponse {
                    balances: balances_map,
                })
        },
        subscription_manager::BalanceEvent::TokenBalanceUpdated { address, balance } => {
            Event::default()
                .event("balance_update")
                .json_data(TokenBalanceSseEvent { address, balance })
        },
        subscription_manager::BalanceEvent::Error { code, message } => {
            Event::default()
                .event("error")
                .json_data(ErrorBalanceSseEvent {
                    code,
                    message,
                })
        }
    }
}

async fn spawn_balances_snapshot_update(ctx: Arc<BalanceContext>, sub: Arc<subscription_manager::Subscription>, snapshot_interval: u64) {
    let cancel = sub.cancel_token.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(snapshot_interval));

        loop {
            tokio::select! {
                _ = cancel.cancelled() => { break; }
                _ = interval.tick() => {
                    update_snapshot_via_multicall(Arc::clone(&ctx), &sub).await;
                }
            }
        }
    });
}

async fn spawn_from_to_erc20_transfer_updates(ctx: Arc<BalanceContext>, sub: Arc<subscription_manager::Subscription>) -> Result<(), RpcError<TransportErrorKind>> {
    let base = Filter::new().event_signature(ERC20::Transfer::SIGNATURE_HASH);
    let filter_to = base.clone().topic1(Topic::from(ctx.owner));
    let filter_from = base.clone().topic2(Topic::from(ctx.owner));

    spawn_balances_transfer_updates(Arc::clone(&ctx), Arc::clone(&sub), filter_to).await?;
    spawn_balances_transfer_updates(Arc::clone(&ctx), Arc::clone(&sub), filter_from).await?;

    Ok(())
}

async fn spawn_balances_transfer_updates(ctx: Arc<BalanceContext>, sub: Arc<subscription_manager::Subscription>, filter: Filter) -> Result<(), RpcError<TransportErrorKind>> {
    let mut ws_stream = ctx.ws_provider
        .clone()
        .subscribe_logs(&filter)
        .await?
        .into_stream();

    let cancel = sub.cancel_token.clone();

    tokio::spawn(async move {
        loop {
            tokio::select!{
                _ = cancel.cancelled() => {
                    break;
                },
                Some(log) = ws_stream.next() => {
                    let token_balance = parse_transfer_and_get_balance(Arc::clone(&ctx), &log).await;
                    let event = match token_balance {
                        Some(balance) => {
                            let balance_as_string = balance.balance.to_string();
                            let mut balances_snapshot = sub.balances_snapshot.write().await;
                            balances_snapshot.insert(balance.address.clone(), balance_as_string.clone());
                            subscription_manager::BalanceEvent::TokenBalanceUpdated {
                                address: balance.address,
                                balance: balance_as_string,
                            }
                        },
                        None => {
                            subscription_manager::BalanceEvent::Error {
                                code: 500,
                                message: "Error when transfer event was parsed".to_string(),
                            }
                        }
                    };

                    let _ = sub.sender.send(event).inspect_err(|err| {
                        tracing::error!("error when send event update token event {err}");
                    });
                }
            }
        }
    });

    Ok(())
}

async fn update_snapshot_via_multicall(ctx: Arc<BalanceContext>, sub: &subscription_manager::Subscription) {
    let result = balances::get_balances(
        &ctx.tokens,
        &ctx.provider,
        ctx.owner,
        ctx.network,
        ctx.multicall3,
    ).await;

    let event = match result {
        Ok(balances) => {
            let mut balances_snapshot = sub.balances_snapshot.write().await;
            *balances_snapshot = balances.clone();
            subscription_manager::BalanceEvent::FullSnapshot(balances)
        },
        Err(e) => {
            tracing::error!("Failed to get balances for {}: {}", ctx.owner, e);
            subscription_manager::BalanceEvent::Error {
                code: 500,
                message: "Error when make multicall3 request".to_string()
            }
        },
    };

    let _ = sub.sender.send(event).inspect_err(|err| {
        tracing::error!("error when send update_snapshot event: {err}");
    });
}


async fn parse_transfer_and_get_balance(ctx: Arc<BalanceContext>, log: &Log) -> Option<TokenBalance> {
    let block_number = log.block_number?;
    
    let decoded_log: Log<ERC20::Transfer> = match log.log_decode() {
        Ok(log) => log,
        Err(_) => return None,
    };

    let erc20 = ERC20::new(decoded_log.address(), &ctx.provider);

    match erc20.balanceOf(ctx.owner).block(block_number.into()).call().await {
        Ok(balance) => Some(TokenBalance {
            address: decoded_log.address(),
            balance,
        }),
        Err(e) => {
            tracing::error!("failed to get balance for {} at block {}: {:?}", decoded_log.address(), block_number, e);
            None
        },
    }
}