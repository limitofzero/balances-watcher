use alloy::transports::http::reqwest;
use serde::Deserialize;
use crate::evm::token::Token;
use crate::evm::networks::EvmNetworks;

#[derive(Debug, Deserialize)]
pub  struct ApiResponse {
    pub tokens: Vec<Token>,
}

pub  async fn fetch_tokens(token_api_url: &String) -> Result<ApiResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(token_api_url).send().await?.json::<ApiResponse>().await?;
    Ok(response)
}