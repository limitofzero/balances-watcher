use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct Args {
    #[arg(long, env = "HTTP_BIND", default_value="0.0.0.0:8080")]
    pub bind: String,

    #[arg(long, env="ARBITRUM_RPC", default_value="")]
    pub arbitrum_rpc: String,

    #[arg(long, env="ETH_RPC", default_value="")]
    pub eth_rpc: String,
}

impl Args {
    pub fn from_env() -> Self {
        Self::parse()
    }
}