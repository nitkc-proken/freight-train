use clap::Parser;
use commands::Args;
use config::Config;

mod api_client;
mod commands;
mod config;
mod nat;
mod transport;

#[tokio::main]
async fn main() {
    let _config = Config::load();
    let args = Args::parse();
    args.run().await;
}
