use std::convert::Infallible;
use std::sync::Arc;
use alloy::transports::http::reqwest;
use axum::{Json, extract::State, extract::Path};
use serde::Deserialize;
use crate::app_state::AppState;
use crate::config::network_config::TokenList;
use crate::evm::networks::EvmNetworks;
use crate::evm::token::Token;

pub async fn get_token_list(Path(network): Path<EvmNetworks>, State(state): State<Arc<AppState>>) -> Json<Vec<Token>> {
    let default_list: Vec<TokenList> = vec![];
    let network_token_list = state
        .network_config
        .token_list(network)
        .unwrap_or(&default_list);

    let mut active_tokens: Vec<Token>  = Vec::new();

    for list in network_token_list {
        match fetch_tokens(&list.source).await {
            Ok(result) => {
                for token in result.tokens {
                    if token.chain_id == network.chain_id() {
                        active_tokens.push(token);
                    }
                }
            },
            Err(e) => println!("error fetching token list from {e} in {}", list.source)
        }
    }

    Json(active_tokens)
}

#[derive(Debug, Deserialize)]
pub  struct ApiResponse {
    pub tokens: Vec<Token>,
}

pub  async fn fetch_tokens(token_api_url: &String) -> Result<ApiResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(token_api_url).send().await?.json::<ApiResponse>().await?;
    Ok(response)
}