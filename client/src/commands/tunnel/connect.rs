use crate::commands::{Args, Command};

#[derive(clap::Parser, Debug)]
pub struct Connect {
    full_network_name: String,
    bind_network_cidr: String,
}

impl Command for Connect {
    async fn run(&self, _args: &Args) {
        println!("Connect");
    }
}