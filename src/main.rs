mod args;

mod evm;
mod token_api;
mod config;

use crate::args::Args;
use config::network_config::{NetworkConfig, TokenList};
use crate::evm::{networks::EvmNetworks, token::Token};
use crate::token_api::fetch_tokens;


#[tokio::main]
async fn main() {
    let cfg = Args::from_env();
    let network_cfg = NetworkConfig::init(&cfg);

    let eth_rpc = match network_cfg
        .rpc_url(EvmNetworks::Eth) {
        Some(url) => url,
        None => "default rpc url"
    };
    println!("eth rpc url is {eth_rpc}");
    println!("bind to {}", cfg.bind);

    let default_list: Vec<TokenList> = vec![];
    let network_token_list = network_cfg.token_list(EvmNetworks::Eth).unwrap_or(&default_list);
    let mut active_tokens: Vec<Token>  = Vec::new();

    for list in network_token_list {
        match fetch_tokens(&list.source).await {
            Ok(result) => {
                active_tokens = [ active_tokens, result.tokens].concat();
            },
            Err(_) => println!("error fetching token list from {}", list.source)
        }
    }

    println!("active tokens: {:?}", active_tokens);
}
