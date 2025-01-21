use super::{Args, Command};
use crate::schema::api::LogoutResponse;
use crate::Config;
use reqwest::{header, Client};
use std::process::exit;
use url::Url;

#[derive(clap::Parser, Debug)]
pub struct Logout {
    server_name: String,
}

impl Command for Logout {
    async fn run(&self, _args: &Args) {
        let config = Config::load();
        let server_name = self.server_name.clone();
        let server = config.servers.get(&server_name).unwrap_or_else(|| {
            eprintln!("{} is not found", server_name);
            exit(1);
        });
        logout(server.token.clone().unwrap(), server.url.clone()).await;
        let mut config = Config::load();
        if config.default.server == Some(server_name.clone()) {
            config.default.server = None;
        }
        config.servers.remove(&server_name);
        config.store();
        println!("Successfully removed {}!", server_name);
    }
}

async fn logout(token: String, mut url: Url) -> LogoutResponse {
    url.set_path(&format!(
        "{}/api/auth/logout",
        url.path().trim_end_matches('/')
    ));
    let response = Client::new()
        .post(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .json(&{})
        .send()
        .await;
    match response {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                eprintln!("Request error: status {}", status.as_u16());
                exit(1);
            }
            println!("{}", status.is_success());
            let response_body = response.json::<LogoutResponse>().await;
            match response_body {
                Ok(response_body) => {
                    if response_body.ok {
                        response_body
                    } else {
                        eprintln!("Error: {}", response_body.message.unwrap());
                        exit(1);
                    }
                }
                Err(ref e) => {
                    eprintln!("Invalid response: {}", e);
                    exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Request error: {}", e);
            exit(1);
        }
    }
}
