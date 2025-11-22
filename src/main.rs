mod args;

mod evm;
mod token_api;
mod config;
mod app_state;

use std::sync::Arc;
use std::net::SocketAddr;
use crate::args::Args;
use config::network_config::{NetworkConfig, TokenList};
use crate::evm::{networks::EvmNetworks, token::Token};
use crate::token_api::fetch_tokens;
use app_state::AppState;
use tokio::net::TcpListener;
use axum::{Json, extract::State, Router, routing::get};

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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Args::from_env();
    let network_cfg = NetworkConfig::init(&cfg);

    let eth_rpc = match network_cfg
        .rpc_url(EvmNetworks::Eth) {
        Some(url) => url,
        None => "default rpc url"
    };
    println!("eth rpc url is {eth_rpc}");


    let app_state = Arc::new(AppState { network_config: Arc::new(network_cfg) });


    let app = Router::new()
        .route("/token-list", get(get_token_list))
        .with_state(app_state);

    let address: SocketAddr = cfg.bind.parse()?;
    println!("Listening to http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
