use thiserror::Error;

use crate::domain::EvmNetwork;

#[derive(Error, Debug, Clone)]
pub enum NetworkConfigError {
    #[error("Invalid chain_id: {0}")]
    InvalidChainId(String),
    #[error("Invalid address of WETH contract for chain: {0}")]
    InvalidAddress(EvmNetwork),
}
