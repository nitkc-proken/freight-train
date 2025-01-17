use super::{Args, Command};
use serde::Serialize;
use serde_json;
use dialoguer::{Input, Password};

#[derive(clap::Parser, Debug)]
pub struct Login {}

impl Command for Login {
    async fn run(&self, _args: &Args) {
        let username = Input::new()
            .with_prompt("username")
            .interact()
            .unwrap();
        let password = Password::new()
            .with_prompt("password")
            .interact()
            .unwrap();
        login(username, password).await;
    }
}

async fn login(user: String, password: String) -> bool {
    let login_request = LoginRequestBody { user, password };
    let login_request_json = serde_json::to_string(&login_request).unwrap();
    println!("{}", login_request_json);
    true // ログインステータスを返す。 現在は便宜上trueを返す
}

#[derive(Debug, Serialize)]
struct LoginRequestBody {
    user: String,
    password: String,
}
