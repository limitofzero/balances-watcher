use std::collections::HashMap;
use alloy::primitives::Address;
use alloy::transports::http::reqwest;
use serde::Deserialize;
use crate::config::network_config::TokenList;
use crate::evm::token::Token;

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    pub tokens: Vec<Token>,
}

pub async fn get_tokens_from_list(token_list: &Vec<TokenList>, network: crate::evm::networks::EvmNetworks) -> HashMap<Address, Token> {
    let mut active_tokens: HashMap<Address, Token>  = HashMap::new();

    for list in token_list {
        match fetch_tokens(&list.source).await {
            Ok(result) => {
                for token in result.tokens {
                    let address = token.address.parse::<Address>().unwrap();
                    if token.chain_id == network.chain_id() {
                        active_tokens.insert(address, token);
                    }
                }
            },
            Err(e) => println!("error fetching token list from {e} in {}", list.source)
        }
    }
    
    active_tokens
}

async fn fetch_tokens(token_api_url: &String) -> Result<ApiResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.get(token_api_url).send().await?.json::<ApiResponse>().await?;
    Ok(response)
}