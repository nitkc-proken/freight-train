use clap::Parser;
use commands::Args;
use config::Config;

mod commands;
mod config;
mod request;
mod schema;

#[tokio::main]
async fn main() {
    let _config = Config::load();
    let args = Args::parse();
    args.run().await;
}
