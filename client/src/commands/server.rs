mod set;
mod remove;

use set::Set;
use remove::Remove;
use super::{Args, Command};

#[derive(clap::Subcommand, Debug)]
pub enum ServerCommands {
    Set(Set),
    Remove(Remove),
}

#[derive(clap::Parser, Debug)]
pub struct Server {
    #[command(subcommand)]
    command: ServerCommands,
}

impl Command for Server {
    async fn run(&self, _args: &Args) {
        println!("{:?}", self);
        match &self.command {
            ServerCommands::Set(set) => set.run(_args).await,
            ServerCommands::Remove(remove) => remove.run(_args).await,
        }
    }
}
