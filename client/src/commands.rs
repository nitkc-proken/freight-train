mod example;
mod login;

use example::Example;
use login::Login;

/// Freight CLI Client
#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Example(Example),
    Login(Login),
}

#[derive(clap::Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, global = true)]
    yes: bool,
    #[arg(short, long, global = true)]
    no: bool,
}

impl Args {
    pub async fn run(&self) {
        match &self.command {
            Commands::Example(example) => example.run(self).await,
            Commands::Login(login) => login.run(self).await,
        }
    }
}

impl Args {
    fn yn(&self) -> Option<bool> {
        match (self.yes, self.no) {
            (_, true) => Some(false),
            (true, _) => Some(true),
            _ => None,
        }
    }
}

pub trait Command {
    async fn run(&self, args: &Args);
}
