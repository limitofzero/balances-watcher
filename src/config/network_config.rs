use crate::args::Args;
use crate::domain::EvmNetwork;
use alloy::primitives::Address;
use std::collections::HashMap;
use std::str::FromStr;

use super::constants::DEFAULT_SNAPSHOT_INTERVAL_SECS;

#[derive(Debug)]
pub struct NetworkConfig {
    pub rpcs: HashMap<EvmNetwork, String>,
    pub ws_rpcs: HashMap<EvmNetwork, String>,
    pub multicall_address: Address,
    pub snapshot_interval: u64,
}

impl NetworkConfig {
    pub fn init(args: &Args) -> Self {
        let mut rpcs: HashMap<EvmNetwork, String> = HashMap::new();
        let mut ws_rpcs: HashMap<EvmNetwork, String> = HashMap::new();

        if !args.arbitrum_rpc.is_empty() {
            rpcs.insert(EvmNetwork::Arbitrum, args.arbitrum_rpc.clone());
        }

        if !args.eth_rpc.is_empty() {
            rpcs.insert(EvmNetwork::Eth, args.eth_rpc.clone());
        }

        if !args.eth_ws_rpc.is_empty() {
            ws_rpcs.insert(EvmNetwork::Eth, args.eth_ws_rpc.clone());
        }

        if !args.sepolia_rpc.is_empty() {
            rpcs.insert(EvmNetwork::Sepolia, args.sepolia_rpc.clone());
        }

        if !args.sepolia_ws_rpc.is_empty() {
            ws_rpcs.insert(EvmNetwork::Sepolia, args.sepolia_ws_rpc.clone());
        }

        let multicall_address = Address::from_str(&args.multicall_address)
            .inspect_err(|err| {
                tracing::error!("Failed to parse multicall_address {}", err);
            })
            .unwrap_or(Address::ZERO);

        let snapshot_interval = args
            .snapshot_interval
            .to_string()
            .parse::<u64>()
            .inspect_err(|err| {
                tracing::warn!("Invalid snapshot interval value: {}", err);
            })
            .unwrap_or(DEFAULT_SNAPSHOT_INTERVAL_SECS);

        Self {
            rpcs,
            multicall_address,
            ws_rpcs,
            snapshot_interval,
        }
    }

    pub fn rpc_url(&self, network: EvmNetwork) -> Option<&String> {
        self.rpcs.get(&network)
    }

    pub fn multicall_address(&self) -> &Address {
        &self.multicall_address
    }
}
