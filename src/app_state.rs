use crate::config::network_config::NetworkConfig;
use crate::domain::EvmNetwork;
use crate::services::subscription_manager::SubscriptionManager;
use crate::services::token_list_fetcher::TokenListFetcher;
use alloy::network::Ethereum;
use alloy::providers::{DynProvider, Provider, ProviderBuilder, WsConnect};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub network_config: Arc<NetworkConfig>,
    pub providers: Arc<HashMap<EvmNetwork, DynProvider<Ethereum>>>,
    pub ws_providers: Arc<HashMap<EvmNetwork, DynProvider>>,
    pub sub_manager: Arc<SubscriptionManager>,
    pub token_list_fetcher: Arc<TokenListFetcher>,
}

impl AppState {
    pub async fn build(network_config: NetworkConfig) -> Arc<Self> {
        let providers = Self::build_rpc_roviders_map(&network_config).await;
        let ws_providers = Self::build_ws_rpc_providers(&network_config).await;

        let sub_manager = Arc::new(SubscriptionManager::new());
        Arc::clone(&sub_manager).spawn_cleanup();

        let token_list_fetcher = Arc::new(TokenListFetcher::new());

        Arc::new(Self {
            network_config: Arc::new(network_config),
            providers: Arc::new(providers),
            ws_providers: Arc::new(ws_providers),
            sub_manager,
            token_list_fetcher,
        })
    }

    async fn build_rpc_roviders_map(
        cfg: &NetworkConfig,
    ) -> HashMap<EvmNetwork, DynProvider<Ethereum>> {
        let mut providers: HashMap<EvmNetwork, DynProvider<Ethereum>> = HashMap::new();

        for network in EvmNetwork::ALL {
            let rpc = &cfg.alchemy_http_url(network);
            match ProviderBuilder::new().connect(rpc).await {
                Ok(provider) => {
                    providers.insert(network, provider.erased());
                    tracing::info!("Provider for network {} is registered", network);
                }
                Err(e) => {
                    tracing::error!("Error to init http rpc connection {:?}", e);
                }
            };
        }

        providers
    }

    async fn build_ws_rpc_providers(cfg: &NetworkConfig) -> HashMap<EvmNetwork, DynProvider> {
        let mut providers: HashMap<EvmNetwork, DynProvider> = HashMap::new();

        for network in EvmNetwork::ALL {
            let rpc = cfg.alchemy_ws_url(network);
            let wc = WsConnect::new(rpc);
            match ProviderBuilder::new().connect_ws(wc).await {
                Ok(provider) => {
                    providers.insert(network, provider.erased());
                }
                Err(e) => {
                    tracing::error!("Error to init ws rpc connection {:?}", e);
                }
            }

            tracing::info!("WS provider for network {} is registered", network);
        }

        providers
    }
}
