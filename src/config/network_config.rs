use std::collections::HashMap;
use serde::Deserialize;
use crate::domain::EvmNetworks;
use crate::args::Args;
use std::fs;
use std::ops::Mul;
use std::str::FromStr;
use alloy::primitives::Address;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TokenList {
    pub priority: u8,
    #[serde(default)]
    pub enabled_by_default: bool,
    pub source: String,
}

pub type TokenListConfig = HashMap<EvmNetworks, Vec<TokenList>>;

#[derive(Debug)]
pub  struct NetworkConfig {
    pub rpcs: HashMap<EvmNetworks, String>,
    pub ws_rpcs: HashMap<EvmNetworks, String>,
    pub multicall_address: Address,
    pub snapshot_interval: u64,
    token_list: HashMap<EvmNetworks, Vec<TokenList>>,
}

const DEFAULT_SNAPSHOT_INTERVAL: u64 = 60;

impl NetworkConfig {
    pub fn init(args: &Args) -> Self {
        let mut rpcs: HashMap<EvmNetworks, String> = HashMap::new();
        let mut ws_rpcs: HashMap<EvmNetworks, String> = HashMap::new();

        if !args.arbitrum_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Arbitrum, args.arbitrum_rpc.clone());
        }

        if !args.eth_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Eth, args.eth_rpc.clone());
        }

        if !args.eth_ws_rpc.is_empty() {
            ws_rpcs.insert(EvmNetworks::Eth, args.eth_ws_rpc.clone());
        }

        if !args.sepolia_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Sepolia, args.sepolia_rpc.clone());
        }

        if !args.sepolia_ws_rpc.is_empty() {
            ws_rpcs.insert(EvmNetworks::Sepolia, args.sepolia_ws_rpc.clone());
        }

        let token_list_config: TokenListConfig = {
          let path = args.token_list_path.clone();
          let content = fs::read_to_string(path).expect("Unable to read token list file");
          serde_json::from_str(content.as_str()).expect("Unable to parse token list file")
        };

        let multicall_address = Address::from_str(&args.multicall_address)
            .inspect_err(|err| {
                tracing::error!("Failed to parse multicall_address {}", err);
            })
            .unwrap_or(Address::ZERO);

        let snapshot_interval =  args.snapshot_interval
            .to_string()
            .parse::<u64>()
            .inspect_err(|err| {
                tracing::warn!("Invalid snapshot interval value: {}", err);
            })
            .unwrap_or(DEFAULT_SNAPSHOT_INTERVAL);

        Self { rpcs, token_list: token_list_config, multicall_address, ws_rpcs, snapshot_interval, }
    }

    pub fn rpc_url(&self, network: EvmNetworks) -> Option<&String> {
        self.rpcs.get(&network)
    }

    pub fn token_list(&self, network: EvmNetworks) -> Option<&Vec<TokenList>> {
        self.token_list.get(&network)
    }
    
    pub fn multicall_address(&self) -> &Address {
        &self.multicall_address
    }
}