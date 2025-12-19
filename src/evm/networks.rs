use std::fmt::{Display, Formatter};
use alloy::primitives::private::derive_more::Display;
use serde::{Deserialize, Deserializer};
use crate::evm::errors::EvmError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u64)]
pub enum EvmNetworks {
    Eth = 1,
    Arbitrum = 42161,
    Sepolia = 11155111,
}

impl EvmNetworks {
    pub fn chain_id(self) -> u64 {
        self as u64
    }
}

impl TryFrom<u64> for EvmNetworks {
    type Error = EvmError;

    fn try_from(id: u64) -> Result<Self, EvmError> {
        match id {
            1 => Ok(EvmNetworks::Eth),
            42161 => Ok(EvmNetworks::Arbitrum),
            11155111 => Ok(EvmNetworks::Sepolia),
            _ => Err(EvmError::UnsupportedNetwork(id)),
        }
    }
}

impl Display for EvmNetworks {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.chain_id())
    }
}

impl<'de> Deserialize<'de> for EvmNetworks {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let id: u64 = s.parse().map_err(serde::de::Error::custom)?;
        EvmNetworks::try_from(id).map_err(serde::de::Error::custom)
    }
}