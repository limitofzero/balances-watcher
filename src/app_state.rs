use std::sync::Arc;
use crate::config::network_config::NetworkConfig;

#[derive(Clone)]
pub struct AppState {
    pub network_config: Arc<NetworkConfig>,
}

