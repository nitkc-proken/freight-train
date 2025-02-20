use super::{Args, Command};
use crate::{api_client::get_api_config, Config};
use openapi::apis::default_api::api_auth_logout_post;
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

async fn logout(token: String, url: Url) -> Option<Option<serde_json::Value>> {
    let mut conf = get_api_config(url.as_str().to_string());
    conf.bearer_access_token = Some(token);
    let response = api_auth_logout_post(&conf).await;
    match response {
        Ok(response) => response.data,
        Err(e) => match e {
            openapi::apis::Error::ResponseError(response_content) => {
                match response_content.entity {
                    Some(e) => match e {
                        openapi::apis::default_api::ApiAuthLogoutPostError::UnknownValue(value) => {
                            eprintln!("Error: {}", value);
                            exit(1);
                        }
                    },
                    None => {
                        eprintln!("Invalid response {}", response_content.status);
                        exit(1);
                    }
                }
            }
            e => {
                eprintln!("Request error: {}", e);
                exit(1);
            }
        },
    }
}
