use connect::Connect;

use super::{Args, Command};

pub mod connect;
#[derive(clap::Parser, Debug)]
pub struct Tunnel {
    #[command(subcommand)]
    sub_command: TunnelSubCommands,
}


#[derive(clap::Subcommand, Debug)]
pub enum TunnelSubCommands {
    Connect(Connect),
}


impl Command for Tunnel {
    async fn run(&self, _args: &Args) {
        match &self.sub_command {
            TunnelSubCommands::Connect(connect) => connect.run(_args).await,
        }
    }
}