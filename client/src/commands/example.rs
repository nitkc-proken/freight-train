use super::{Args, Command};

#[derive(clap::Parser, Debug)]
pub struct Example {
    #[arg(short, long)]
    name: String,
}

impl Command for Example {
    async fn run(&self, _args: &Args) {
        println!("Hello, world! {:?}", self.name);
    }
}
