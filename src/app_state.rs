use std::collections::HashMap;
use std::sync::Arc;
use alloy::network::Ethereum;
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use crate::config::network_config::NetworkConfig;
use crate::evm::networks::EvmNetworks;

#[derive(Clone)]
pub struct AppState {
    pub network_config: Arc<NetworkConfig>,
    pub providers: Arc<HashMap<EvmNetworks, DynProvider<Ethereum>>>,
}

impl AppState {
    pub async fn build(network_config: NetworkConfig) -> Arc<Self> {
        let mut providers: HashMap<EvmNetworks, DynProvider<Ethereum>>  = HashMap::new();

        for (network, rpc) in &network_config.rpcs {
            if rpc.is_empty() { continue; }

            match ProviderBuilder::new().connect(&rpc).await {
                Ok(provider) => {
                    providers.insert(network.clone(), provider.erased());
                },
                Err(e) => {
                    println!("provider error: {}", e);
                },
            };
        }

        Arc::new(Self { network_config: Arc::new(network_config), providers: Arc::new(providers) })
    }
}