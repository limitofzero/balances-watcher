mod api;
mod app_error;
mod app_state;
mod args;
mod config;
mod domain;
mod evm;
mod routes;
mod services;
mod tracing;

use crate::args::Args;
use crate::routes::create_router::create_router;
use crate::tracing::init_tracing::init_tracing;
use app_state::AppState;
use config::network_config::NetworkConfig;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let cfg = Args::from_env();
    if cfg.alchemy_api_key.is_empty() {
        return Err("ALCHEMY_API_KEY is required".into());
    }

    let network_cfg = NetworkConfig::init(&cfg);

    let allowed_origins = network_cfg.allowed_origins.clone();
    let app_state = AppState::build(network_cfg).await;
    let app = create_router(app_state, allowed_origins);

    let address: SocketAddr = cfg.bind.parse()?;
    ::tracing::info!("Listening to http://{}", address);

    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
