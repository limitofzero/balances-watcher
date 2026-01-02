use alloy::network::Ethereum;
use alloy::providers::RootProvider;
use std::sync::Arc;

pub type EthProvider = RootProvider<Ethereum>;

pub fn create_provider(rpc_url: &str) {}
