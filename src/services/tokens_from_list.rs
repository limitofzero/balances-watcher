use std::{collections::HashSet, time::Instant};

use crate::config::constants::TOKEN_FETCH_CONCURRENCY;
use crate::domain::{EvmNetwork, Token};
use crate::services::errors::ServiceError;
use alloy::primitives::Address;
use alloy::transports::http::{reqwest, Client};
use futures::{stream, StreamExt};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub tokens: Vec<Token>,
}

pub async fn get_tokens_from_list(
    token_list: &Vec<String>,
    network: EvmNetwork,
) -> Result<HashSet<Address>, ServiceError> {
    let mut active_tokens: HashSet<Address> = HashSet::new();

    let t0 = Instant::now();

    let client = Client::new();

    let mut stream = stream::iter(token_list.iter().cloned())
        .map(move |list| {
            let client = client.clone();
            async move {
                let source = list.clone();
                let result = fetch_tokens(&client, &source).await;
                (source, result)
            }
        })
        .buffer_unordered(TOKEN_FETCH_CONCURRENCY);

    while let Some((source, response)) = stream.next().await {
        match response {
            Ok(result) => {
                for token in result.tokens {
                    let address = token.address.parse::<Address>().unwrap();
                    if token.chain_id == network.chain_id() {
                        active_tokens.insert(address);
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "get_tokens_from_list: failed to fetch tokens from list({source}): {:?}",
                    e
                );

                return Err(ServiceError::UnableToLoadList(source));
            }
        }
    }

    tracing::info!(time = t0.elapsed().as_millis(), "finished fetching tokens");

    Ok(active_tokens)
}

async fn fetch_tokens(
    client: &Client,
    token_api_url: &String,
) -> Result<ApiResponse, reqwest::Error> {
    let response = client
        .get(token_api_url)
        .send()
        .await?
        .json::<ApiResponse>()
        .await?;
    Ok(response)
}
