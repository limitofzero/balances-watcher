use std::{convert::Infallible, collections::HashMap, time::Duration, sync::Arc};
use axum::{response::sse::{Sse, Event}, extract::{Path, State}, http::StatusCode};
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{ primitives::Address};
use alloy::providers::DynProvider;
use alloy::pubsub::ConnectionHandle;
use crate::config::network_config::TokenList;
use crate::services::{balances, tokens_from_list};
use futures::{Stream, StreamExt};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;
use crate::evm::token::Token;

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

struct BalanceContext {
    owner: Address,
    provider: DynProvider,
    tokens: HashMap<Address, Token>,
}

#[derive(Serialize)]
struct BalanceStreamError {
    error: String,
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
        Some(_) => provider.clone(),
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
    });

    let interval = interval(Duration::from_secs(60));

    let multicall_interval_handle = IntervalStream::new(interval)
        .then(move |_| {
            let ctx = Arc::clone(&ctx);
            
            async move {
                let result = balances::get_balances(
                    &ctx.tokens,
                    &ctx.provider,
                    ctx.owner
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

                Ok(event)
            }
        });

    Ok(Sse::new(multicall_interval_handle))
}