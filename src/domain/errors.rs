use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum EvmError {
    #[error("Network with id {0} is not supported")]
    UnsupportedNetwork(u64),

    #[error("Network id should be integer")]
    InvalidNetworkId,
}
