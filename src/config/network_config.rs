use crate::domain::EvmNetwork;
use crate::{args::Args, config::errors::NetworkConfigError};
use alloy::primitives::Address;
use std::collections::HashMap;
use std::str::FromStr;

use super::constants::{DEFAULT_MAX_WATCHED_TOKENS_LIMIT, DEFAULT_SNAPSHOT_INTERVAL_SECS};

#[derive(Debug)]
pub struct NetworkConfig {
    api_key: String,
    pub multicall_address: Address,
    pub snapshot_interval: usize,
    pub max_watched_tokens_limit: usize,
    pub allowed_origins: Vec<String>,
    pub weth_addresses: HashMap<EvmNetwork, Address>,
}

impl NetworkConfig {
    pub fn init(args: &Args) -> Self {
        let api_key = args.alchemy_api_key.clone();

        let multicall_address = Address::from_str(&args.multicall_address)
            .inspect_err(|err| {
                tracing::error!("Failed to parse multicall_address {}", err);
            })
            .unwrap_or(Address::ZERO);

        let snapshot_interval: usize = args
            .snapshot_interval
            .parse()
            .inspect_err(|err| {
                tracing::warn!("Invalid snapshot interval value: {}", err);
            })
            .unwrap_or(DEFAULT_SNAPSHOT_INTERVAL_SECS);

        let max_watched_tokens_limit: usize = args
            .max_watched_tokens_limit
            .parse()
            .inspect_err(|err| {
                tracing::warn!("Invalid MAX_WATCHED_TOKENS_LIMIT value: {}", err);
            })
            .unwrap_or(DEFAULT_MAX_WATCHED_TOKENS_LIMIT);

        let allowed_origins: Vec<String> = args
            .allowed_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        tracing::info!(origins = %allowed_origins.join(", "), "init origins from env");

        let weth_addresses = Self::parse_weth_contracts_map(&args.weth_contract_addresses);

        Self {
            api_key,
            multicall_address,
            snapshot_interval,
            max_watched_tokens_limit,
            allowed_origins,
            weth_addresses,
        }
    }

    pub fn multicall_address(&self) -> &Address {
        &self.multicall_address
    }

    fn parse_weth_contracts_map(weth_contract_addresses: &String) -> HashMap<EvmNetwork, Address> {
        let mut weth_address_map: HashMap<EvmNetwork, Address> = HashMap::new();

        for entry in weth_contract_addresses.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                tracing::error!("WETH_CONTRACT_ADDRESSES should contain <network:address>,<network:address> list");
                continue;
            }

            if let Some((chain_id, address)) = entry.split_once(':') {
                match NetworkConfig::parse_network_address(chain_id, address) {
                    Ok((network, address)) => {
                        weth_address_map.insert(network, address);

                        tracing::info!(
                            "init address({}) for WETH for network({})",
                            address,
                            network
                        );
                    }
                    Err(err) => {
                        tracing::error!(
                            error = %err,
                            "error when parse network:address value",
                        );
                    }
                }
            } else {
                tracing::error!("WETH_CONTRACT_ADDRESSES should contain <network:address>,<network:address> list");
            }
        }

        weth_address_map
    }

    fn network_subdomain(network: EvmNetwork) -> &'static str {
        match network {
            EvmNetwork::Eth => "eth-mainnet",
            EvmNetwork::Arbitrum => "arb-mainnet",
            EvmNetwork::Sepolia => "eth-sepolia",
        }
    }

    pub fn alchemy_http_url(&self, network: EvmNetwork) -> String {
        let subdomain = Self::network_subdomain(network);
        format!("https://{}.g.alchemy.com/v2/{}", subdomain, self.api_key)
    }

    pub fn alchemy_ws_url(&self, network: EvmNetwork) -> String {
        let subdomain = Self::network_subdomain(network);
        format!("wss://{}.g.alchemy.com/v2/{}", subdomain, self.api_key)
    }

    fn parse_network_address(
        chain_id: &str,
        address: &str,
    ) -> Result<(EvmNetwork, Address), NetworkConfigError> {
        let network = chain_id
            .parse::<EvmNetwork>()
            .map_err(|err| NetworkConfigError::InvalidChainId(err.to_string()))?;

        let address = address
            .parse::<Address>()
            .map_err(|_| NetworkConfigError::InvalidAddress(network))?;

        Ok((network, address))
    }
}
