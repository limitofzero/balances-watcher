use thiserror::Error;

use crate::{config::errors, domain::EvmNetwork};

#[derive(Error, Debug, Clone)]
pub enum NetworkConfigError {
    #[error("Invalid WETH network format, should be <chain_id>:<address>")]
    InvalidWethNetworkFormat,

    #[error("Invalid chain_id: {0}")]
    InvalidChainId(String),

    #[error("Invalid address of WETH contract for chain: {0}")]
    InvalidAddress(EvmNetwork),
}
