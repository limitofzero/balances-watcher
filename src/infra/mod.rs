use std::sync::Arc;
use alloy::network::Ethereum;
use alloy::providers::RootProvider;

pub type EthProvider = RootProvider<Ethereum>;

pub fn create_provider(rpc_url: &str)  {

}