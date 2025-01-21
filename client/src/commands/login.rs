use super::{Args, Command};
use crate::config::{Config, ServerConfig};
use crate::request::Post;
use crate::schema::api::{LoginRequest, LoginResponse};
use dialoguer::{Confirm, Input, Password};
use std::process::exit;
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
            token: Some(response_body.data.unwrap().token.token),
        };
        config.servers.insert(self.server_name.clone(), server);
        if self.default {
            config.default.server = Some(self.server_name.clone());
        }
        config.store();
        println!("Logiln Successful!")
    }
}

async fn login(username: String, password: String, mut url: Url) -> LoginResponse {
    let login_request = LoginRequest { username, password };
    url.set_path(&format!(
        "{}/api/auth/login",
        url.path().trim_end_matches('/')
    ));
    let response = Post::new(url, &login_request).send().await;
    match response {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() && status.as_u16() != 401 {
                eprintln!("Request error: status {}", status.as_u16());
                eprintln!("Please check your internet connection or server URL and try again.")
            }
            let response_body = response.json::<LoginResponse>().await;
            match response_body {
                Ok(response_body) => {
                    if response_body.ok {
                        response_body
                    } else {
                        eprintln!("Login Failed!");
                        eprintln!("Wrong username or password.");
                        eprintln!("Check your credentials.");
                        exit(1);
                    }
                }
                Err(e) => {
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
