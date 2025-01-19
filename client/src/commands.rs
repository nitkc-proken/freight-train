mod example;
mod login;
mod server;

use example::Example;
use login::Login;
use server::Server;

/// Freight CLI Client
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Example(Example),
    Login(Login),
    Server(Server),
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

impl Args {
    pub async fn run(&self) {
        match &self.command {
            Commands::Example(example) => example.run(self).await,
            Commands::Login(login) => login.run(self).await,
            Commands::Server(server) => server.run(self).await,
        }
    }
}

pub trait Command {
    async fn run(&self, args: &Args);
}
