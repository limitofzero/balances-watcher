use crate::domain::EvmNetwork;
use alloy::primitives::{address, Address};

pub fn get_wrapped_address(network: &EvmNetwork) -> Address {
    match network {
        EvmNetwork::Eth => address!("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
        EvmNetwork::Sepolia => address!("0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14"),
        EvmNetwork::Arbitrum => address!("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
    }
}
