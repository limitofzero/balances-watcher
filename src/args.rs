use clap::Parser;

const DEFAULT_TOKEN_LIST_PATH: &str = "configs/tokens_list.json";

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, env = "HTTP_BIND", default_value = "0.0.0.0:8080")]
    pub bind: String,

    #[arg(long, env = "ARBITRUM_RPC", default_value = "")]
    pub arbitrum_rpc: String,

    #[arg(long, env = "ETH_RPC", default_value = "")]
    pub eth_rpc: String,

    #[arg(long, env = "ETH_WC_RPC", default_value = "")]
    pub eth_ws_rpc: String,

    #[arg(long, env = "SEPOLIA_RPC", default_value = "")]
    pub sepolia_rpc: String,

    #[arg(long, env = "SEPOLIA_WC_RPC", default_value = "")]
    pub sepolia_ws_rpc: String,

    #[arg(long, env="TOKEN_LIST_PATH", default_value=DEFAULT_TOKEN_LIST_PATH)]
    pub token_list_path: String,

    #[arg(long, env = "MULTICALL_ADDRESS", default_value = "")]
    pub multicall_address: String,

    #[arg(long, env = "SNAPSHOT_INTERVAL", default_value = "60")]
    pub snapshot_interval: String,

    #[arg(long, env = "MAX_WATCHED_TOKENS_LIMIT", default_value = "1000")]
    pub max_watched_tokens_limit: String,

    #[arg(long, env = "ALLOWED_ORIGINS", default_value = "")]
    allowed_origins_from_env: String,
}

impl Args {
    pub fn from_env() -> Self {
        Self::parse()
    }

    pub fn allowed_origins(&self) -> Vec<String> {
        self.allowed_origins_from_env
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
