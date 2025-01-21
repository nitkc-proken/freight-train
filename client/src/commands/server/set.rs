use crate::commands::server::{Args, Command};
use crate::config::{Config, ServerConfig};

#[derive(clap::Parser, Debug)]
pub struct Set {
    #[arg(short, long)]
    default: bool,
    name: String,
    url: url::Url,
}

impl Command for Set {
    async fn run(&self, _args: &Args) {
        let mut config = Config::load();
        config.servers.insert(self.name.clone(),ServerConfig{url:self.url.clone(),token:None});
        if self.default {
            config.default.server = Some(self.name.clone());
        }
        config.store();
    }
}
