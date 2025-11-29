mod args;

mod evm;
mod config;
mod app_state;
mod routes;
mod api;
mod services;
mod infra;

use std::sync::Arc;
use std::net::SocketAddr;
use crate::args::Args;
use config::network_config::{NetworkConfig};
use crate::evm::{networks::EvmNetworks};
use app_state::AppState;
use tokio::net::TcpListener;
use crate::routes::create_router::create_router;

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


    let app_state = AppState::build(network_cfg).await;
    let app = create_router(app_state);

    let address: SocketAddr = cfg.bind.parse()?;
    println!("Listening to http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
