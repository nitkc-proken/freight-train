use crate::commands::server::{Args, Command};

#[derive(clap::Parser, Debug)]
pub struct Remove {
    name: String,
}

impl Command for Remove {
    async fn run(&self, _args: &Args) {
        println!("{:?}", self);
    }
}
