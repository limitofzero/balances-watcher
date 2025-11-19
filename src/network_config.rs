use std::collections::HashMap;
use crate::evm_networks::EvmNetworks;
use crate::args::Args;

pub  struct NetworkConfig {
    rpcs: HashMap<EvmNetworks, String>
}

impl NetworkConfig {
    pub fn from_args(args: &Args) -> Self {
        let mut rpcs: HashMap<EvmNetworks, String> = HashMap::new();

        if !args.arbitrum_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Arbitrum, args.arbitrum_rpc.clone());
        }

        if !args.eth_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Eth, args.eth_rpc.clone());
        }

        Self { rpcs: HashMap::new() }
    }

    pub fn rpc_url(&self, network: EvmNetworks) -> Option<&String> {
        self.rpcs.get(&network)
    }
}