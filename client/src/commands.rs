mod example;
mod login;

use example::Example;
use login::Login;

/// Freight CLI Client
#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Example(Example),
    Login(Login),
}

impl Args {
    pub async fn run(&self) {
        match &self.command {
            Commands::Example(example) => example.run(self).await,
            Commands::Login(login) => login.run(self).await,
        }
    }
}

pub trait Command {
    async fn run(&self, args: &Args);
}
