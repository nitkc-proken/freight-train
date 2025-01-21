use crate::commands::server::{Args, Command};
use crate::config::Config;
#[derive(clap::Parser, Debug)]
pub struct Remove {
    name: String,
}

impl Command for Remove {
    async fn run(&self, _args: &Args) {
        let mut config = Config::load();
        config.servers.remove(&self.name);
        if config.default.server == Some(self.name.clone()) {
            config.default.server = None;
        }
        config.store();
    }
}
