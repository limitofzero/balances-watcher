mod args;

use crate::args::Args;

#[tokio::main]
async fn main() {
    let cfg = Args::from_env();

    println!("eth rpc url is {}", cfg.eth_rpc);
    println!("bind to {}", cfg.bind);
}
