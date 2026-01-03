use crate::domain::errors::EvmError;
use alloy::primitives::{address, Address};
use serde::{Deserialize, Deserializer};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u64)]
pub enum EvmNetwork {
    Eth = 1,
    Arbitrum = 42161,
    Sepolia = 11155111,
}

const NATIVE_ADDRESS: Address = address!("0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE");

impl EvmNetwork {
    pub fn chain_id(self) -> u64 {
        self as u64
    }

    pub fn native_token_address(self) -> Address {
        NATIVE_ADDRESS
    }
}

impl TryFrom<u64> for EvmNetwork {
    type Error = EvmError;

    fn try_from(id: u64) -> Result<Self, EvmError> {
        match id {
            1 => Ok(EvmNetwork::Eth),
            42161 => Ok(EvmNetwork::Arbitrum),
            11155111 => Ok(EvmNetwork::Sepolia),
            _ => Err(EvmError::UnsupportedNetwork(id)),
        }
    }
}

impl Display for EvmNetwork {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.chain_id())
    }
}

impl<'de> Deserialize<'de> for EvmNetwork {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let id: u64 = s.parse().map_err(serde::de::Error::custom)?;
        EvmNetwork::try_from(id).map_err(serde::de::Error::custom)
    }
}
