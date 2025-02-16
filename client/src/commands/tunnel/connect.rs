use cidr::{Cidr, Ipv4Cidr};

use crate::commands::{Args, Command};

#[derive(clap::Parser, Debug)]
pub struct Connect {
    full_network_name: String,
    bind_network_cidr: Ipv4Cidr ,
}

impl Command for Connect {
    async fn run(&self, _args: &Args) {
        // Validate the network name
        let parts: Vec<&str> = self.full_network_name.split('/').collect();
        if parts.len() != 2 {
            eprintln!("Invalid network name format. Expected format: 'owner/name'");
            return;
        }
        let owner_user_name = parts[0];
        let network_name = parts[1];

        // get network
        let network = match get_network(owner_user_name, network_name).await {
            Ok(network) => network,
            Err(e) => {
                eprintln!("Failed to get network: {}", e);
                return;
            }
        };

    }
}
