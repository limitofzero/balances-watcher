use std::sync::Arc;
use alloy::transports::http::reqwest;
use axum::{Json, extract::State};
use serde::Deserialize;
use crate::app_state::AppState;
use crate::config::network_config::TokenList;
use crate::evm::networks::EvmNetworks;
use crate::evm::token::Token;

pub async fn get_token_list(State(state): State<Arc<AppState>>) -> Json<Vec<Token>> {
    let default_list: Vec<TokenList> = vec![];
    let network_token_list = state
        .network_config
        .token_list(EvmNetworks::Eth)
        .unwrap_or(&default_list);

    let mut active_tokens: Vec<Token>  = Vec::new();

    for list in network_token_list {
        match fetch_tokens(&list.source).await {
            Ok(result) => {
                active_tokens = [ active_tokens, result.tokens].concat();
            },
            Err(_) => println!("error fetching token list from {}", list.source)
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