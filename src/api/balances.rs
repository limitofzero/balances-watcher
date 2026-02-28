use crate::api::errors::StreamError;
use crate::app_state::AppState;
use crate::domain::{BalanceEvent, EvmNetwork, SubscriptionKey};
use crate::services::cleanup_stream;
use crate::services::watcher::{Watcher, WatcherContext};
use alloy::primitives::Address;
use axum::{
    extract::{Path, State},
    response::sse::{Event, Sse},
};
use futures::{Stream, StreamExt};
use serde::Serialize;
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio_stream::wrappers::BroadcastStream;

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

#[derive(Serialize)]
struct ErrorBalanceSseEvent {
    code: u16,
    message: String,
}

pub async fn get_balances(
    Path((network, owner)): Path<(EvmNetwork, Address)>,
    State(state): State<Arc<AppState>>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StreamError> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider.clone(),
        None => {
            return Err(StreamError {
                code: 404,
                message: format!("No provider for network {}", network),
            })
        }
    };

    let ws_provider = match state.ws_providers.get(&network) {
        None => {
            return Err(StreamError {
                code: 404,
                message: format!("No ws provider for network {}", network),
            });
        }
        Some(ws_provider) => ws_provider.clone(),
    };

    let multicall3 = state.network_config.multicall_address();
    if multicall3.is_empty() {
        return Err(StreamError {
            code: 404,
            message: format!("No multicall3 for network {}", network),
        });
    }

    let sub_key = SubscriptionKey { owner, network };

    let (rx, is_first, subscription) =
        state
            .sub_manager
            .subscribe(sub_key.clone())
            .await
            .map_err(|e| StreamError {
                code: 500,
                message: e.to_string(),
            })?;

    let weth_address = state.network_config.weth_address(&network);

    if is_first {
        let ctx = WatcherContext {
            provider,
            owner,
            network,
            multicall3: *multicall3,
            ws_provider,
            weth_address,
        };

        Watcher::new(ctx, Arc::clone(&subscription))
            .spawn_watchers(state.network_config.snapshot_interval)
            .await;
    } else {
        let balance_snapshot = subscription.balances_snapshot.read().await;

        let event = if balance_snapshot.is_empty() {
            BalanceEvent::Error {
                code: 500,
                message: format!("Empty snapshot for {network} for {owner}"),
            }
        } else {
            let balance_snapshot: HashMap<Address, String> = balance_snapshot
                .clone()
                .into_iter()
                .map(|(address, balance)| (address, balance.to_string()))
                .collect();
            BalanceEvent::BalanceUpdate(balance_snapshot)
        };

        let _ = subscription.sender.send(event).inspect_err(|err| {
            tracing::error!(
                error = %err,
                "error when send balance_snapshot update for new client {} network {}",
                owner,
                network,
            );
        });
    }

    let manager_for_cleanup = Arc::clone(&state.sub_manager);
    let key_for_cleanup = sub_key.clone();

    let sse_stream = BroadcastStream::new(rx).filter_map(|result| async move {
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
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "broadcast stream error",
                );
                None
            }
        }
    });

    let cleanup_stream =
        cleanup_stream::CleanupStream::new(sse_stream, manager_for_cleanup, key_for_cleanup);

    Ok(Sse::new(cleanup_stream))
}

fn balance_event_to_sse(event: BalanceEvent) -> Result<Event, axum::Error> {
    match event {
        BalanceEvent::BalanceUpdate(balances_map) => Event::default()
            .event("balance_update")
            .json_data(BalancesResponse {
                balances: balances_map,
            }),
        BalanceEvent::Error { code, message } => Event::default()
            .event("error")
            .json_data(ErrorBalanceSseEvent { code, message }),
    }
}
