use crate::commands::server::{Args, Command};

#[derive(clap::Parser, Debug)]
pub struct Set {
    #[arg(short, long)]
    default: bool,
    name: String,
    url: url::Url,
}

impl Command for Set {
    async fn run(&self, _args: &Args) {
        println!("{:?}", self);
    }
}
