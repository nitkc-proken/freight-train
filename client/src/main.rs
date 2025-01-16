use clap::Parser;
use commands::Args;

mod commands;
#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.run().await;
}
