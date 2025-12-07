use std::{convert::Infallible, collections::HashMap, time::Duration, sync::Arc};
use axum::{Json, response::sse::{Sse, Event}, extract::{Path, State}, http::StatusCode};
use serde::Serialize;
use crate::app_state::AppState;
use crate::evm::networks::EvmNetworks;
use alloy::{ primitives::Address};
use alloy::providers::DynProvider;
use crate::config::network_config::TokenList;
use crate::evm::{errors::EvmError};
use crate::services::balances;
use futures::{SinkExt, Stream, StreamExt};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

#[derive(Serialize)]
pub struct BalancesResponse {
    pub balances: HashMap<Address, String>,
}

struct BalanceContext {
    owner: Address,
    provider: DynProvider,
    network: EvmNetworks,
    tokens_list: Vec<TokenList>,
}

#[derive(Serialize)]
struct BalanceStreamError {
    error: String,
}

pub async fn get_balances(Path((network, owner)): Path<(EvmNetworks, Address)>, State(state): State<Arc<AppState>>) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let provider = match state.providers.get(&network) {
        Some(provider) => provider.clone(),
        None => return Err(StatusCode::NOT_FOUND),
    };

    let network_token_list: Vec<TokenList> = state
        .network_config
        .token_list(network)
        .cloned()
        .unwrap_or_default();

    let ctx = Arc::new(BalanceContext {
        provider,
        network,
        tokens_list: network_token_list,
        owner,
    });

    let interval = interval(Duration::from_secs(8));

    let stream = IntervalStream::new(interval)
        .then(move |_| {
            let ctx = Arc::clone(&ctx);
            
            async move {
                let result = balances::get_balances(
                    &ctx.tokens_list,
                    &ctx.provider,
                    ctx.network,
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

    Ok(Sse::new(stream))
}