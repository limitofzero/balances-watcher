mod args;
mod network_config;

mod evm_networks;

use crate::args::Args;
use crate::network_config::NetworkConfig;
use evm_networks::EvmNetworks;

#[tokio::main]
async fn main() {
    let cfg = Args::from_env();
    let network_cfg = NetworkConfig::from_args(&cfg);

    let eth_rpc = network_cfg
        .rpc_url(EvmNetworks::Eth)
        .ok_or("no eth rpc url")
        .unwrap();
    println!("eth rpc url is {eth_rpc}");
    println!("bind to {}", cfg.bind);
}
