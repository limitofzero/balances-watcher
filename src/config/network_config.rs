use std::collections::HashMap;
use serde::Deserialize;
use crate::evm::networks::EvmNetworks;
use crate::evm::token::Token;
use crate::args::Args;
use std::fs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenList {
    pub priority: u8,
    #[serde(default)]
    pub enabled_by_default: bool,
    pub source: String,
}

pub type TokenListConfig = HashMap<EvmNetworks, Vec<TokenList>>;

pub  struct NetworkConfig {
    rpcs: HashMap<EvmNetworks, String>,
    token_list: HashMap<EvmNetworks, Vec<TokenList>>,
}

impl NetworkConfig {
    pub fn init(args: &Args) -> Self {
        let mut rpcs: HashMap<EvmNetworks, String> = HashMap::new();

        if !args.arbitrum_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Arbitrum, args.arbitrum_rpc.clone());
        }

        if !args.eth_rpc.is_empty() {
            rpcs.insert(EvmNetworks::Eth, args.eth_rpc.clone());
        }

        let token_list_config: TokenListConfig = {
          let path = args.token_list_path.clone();
          let content = fs::read_to_string(path).expect("Unable to read token list file");
          serde_json::from_str(content.as_str()).expect("Unable to parse token list file")
        };


        Self { rpcs, token_list: token_list_config }
    }

    pub fn rpc_url(&self, network: EvmNetworks) -> Option<&String> {
        self.rpcs.get(&network)
    }
    
    pub fn token_list(&self, network: EvmNetworks) -> Option<&Vec<TokenList>> {
        self.token_list.get(&network)
    }
}