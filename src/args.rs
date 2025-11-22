use clap::Parser;

const DEFAULT_TOKEN_LIST_PATH: &str = "configs/tokens_list.json";

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, env = "HTTP_BIND", default_value="0.0.0.0:8080")]
    pub bind: String,

    #[arg(long, env="ARBITRUM_RPC", default_value="")]
    pub arbitrum_rpc: String,

    #[arg(long, env="ETH_RPC", default_value="")]
    pub eth_rpc: String,

    #[arg(long, env="TOKEN_LIST_PATH", default_value=DEFAULT_TOKEN_LIST_PATH)]
    pub token_list_path: String,
}

impl Args {
    pub fn from_env() -> Self {
        Self::parse()
    }
}