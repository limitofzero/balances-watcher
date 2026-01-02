use crate::config::network_config::NetworkConfig;
use crate::domain::EvmNetworks;
use crate::services::subscription_manager::SubscriptionManager;
use alloy::network::Ethereum;
use alloy::providers::{DynProvider, Provider, ProviderBuilder, WsConnect};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub network_config: Arc<NetworkConfig>,
    pub providers: Arc<HashMap<EvmNetworks, DynProvider<Ethereum>>>,
    pub ws_providers: Arc<HashMap<EvmNetworks, DynProvider>>,
    pub sub_manager: Arc<SubscriptionManager>,
}

impl AppState {
    pub async fn build(network_config: NetworkConfig) -> Arc<Self> {
        let providers = Self::build_rpc_roviders_map(&network_config.rpcs).await;
        let ws_providers = Self::build_ws_rpc_providers(&network_config.ws_rpcs).await;

        Arc::new(Self {
            network_config: Arc::new(network_config),
            providers: Arc::new(providers),
            ws_providers: Arc::new(ws_providers),
            sub_manager: Arc::new(SubscriptionManager::new()),
        })
    }

    async fn build_rpc_roviders_map(
        rpcs: &HashMap<EvmNetworks, String>,
    ) -> HashMap<EvmNetworks, DynProvider<Ethereum>> {
        let mut providers: HashMap<EvmNetworks, DynProvider<Ethereum>> = HashMap::new();

        for (network, rpc) in rpcs {
            if rpc.is_empty() {
                continue;
            }

            match ProviderBuilder::new().connect(&rpc).await {
                Ok(provider) => {
                    providers.insert(network.clone(), provider.erased());
                    tracing::info!("Provider for network {} is registered", network);
                }
                Err(e) => {
                    tracing::warn!(
                        error = e.to_string().as_str(),
                        "RPC http provider failed to connect"
                    );
                }
            };
        }

        providers
    }

    async fn build_ws_rpc_providers(
        ws_rpcs: &HashMap<EvmNetworks, String>,
    ) -> HashMap<EvmNetworks, DynProvider> {
        let mut providers: HashMap<EvmNetworks, DynProvider> = HashMap::new();

        for (network, ws_rpc) in ws_rpcs {
            if ws_rpc.is_empty() {
                continue;
            }

            let wc = WsConnect::new(ws_rpc);
            match ProviderBuilder::new().connect_ws(wc).await {
                Ok(provider) => {
                    providers.insert(network.clone(), provider.erased());
                }
                Err(e) => {
                    tracing::error!("Error to init ws connection {:?}", e);
                }
            }

            tracing::info!("WS provider for network {} is registered", network);
        }

        providers
    }
}
