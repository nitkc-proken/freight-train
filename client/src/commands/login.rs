use super::{Args, Command};
use crate::{
    api_client::get_api_config,
    config::{Config, ServerConfig},
};
use dialoguer::{Confirm, Input, Password};
use reqwest::Client;
use std::{ops::Deref, process::exit};
use url::Url;

#[derive(clap::Parser, Debug)]
pub struct Login {
    #[arg(short, long)]
    default: bool,
    server_name: String,
    url: Url,
}

impl Command for Login {
    async fn run(&self, _args: &Args) {
        let config = Config::load();
        let mut url = self.url.clone();
        if url.cannot_be_a_base() {
            eprintln!("Error: Invalid URL");
            exit(1);
        }
        if config.servers.contains_key(&self.server_name.clone()) {
            let answer = _args.yn().unwrap_or_else(|| -> bool {
                println!("Server {} already has an active login.", self.server_name);
                Confirm::new()
                    .with_prompt("Do you want to update the cerdentials?")
                    .default(false)
                    .interact()
                    .unwrap()
            });
            if !answer {
                return;
            }
        }
        if config.default.server != None && self.default {
            let answer = _args.yn().unwrap_or_else(|| -> bool {
                println!(
                    "Server {} already set as default.",
                    config.default.server.unwrap()
                );
                Confirm::new()
                    .with_prompt("Do yo want to update the default server?")
                    .default(true)
                    .interact()
                    .unwrap()
            });
            if !answer {
                return;
            }
        }
        url.set_path(url.path().to_string().trim_end_matches('/'));
        url.set_fragment(None);
        url.set_query(None);
        let username = Input::new().with_prompt("username").interact().unwrap();
        let password = Password::new().with_prompt("password").interact().unwrap();
        let response_body = login(username, password, url.clone()).await;
        let mut config = Config::load();
        let server = ServerConfig {
            url,
            token: Some(response_body.token.token),
        };
        config.servers.insert(self.server_name.clone(), server);
        if self.default {
            config.default.server = Some(self.server_name.clone());
        }
        config.store();
        println!("Logiln Successful!")
    }
}

async fn login(username: String, password: String, url: Url) -> openapi::models::UserWithTokenResponse{
    let login_credential = openapi::models::LoginCredential { username, password };
    let conf = get_api_config(url.as_str().to_string());

    let response =
        openapi::apis::default_api::api_auth_login_post(&conf, Some(login_credential)).await;
    match response {
        Ok(response) => *response.data,
        Err(e) => match e {
            openapi::apis::Error::ResponseError(response_content) => {
                match response_content.entity {
                    Some((e)) => match e {
                        openapi::apis::default_api::ApiAuthLoginPostError::UnknownValue(_value) => {
                            eprintln!("Login Failed!");
                            eprintln!("Wrong username or password.");
                            eprintln!("Check your credentials.");
                            exit(1);
                        }
                    },
                    None => {
                        eprintln!("Invalid Response");
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
