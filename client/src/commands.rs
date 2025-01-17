mod example;

use example::Example;

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
}

impl Args {
    pub async fn run(&self) {
        match &self.command {
            Commands::Example(example) => example.run(self).await,
        }
    }
}

pub trait Command {
    async fn run(&self, args: &Args);
}
