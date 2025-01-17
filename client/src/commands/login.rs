use super::{Args, Command};
use proconio::input;
use serde::Serialize;
use serde_json;
use std::io::{self, Write};

#[derive(clap::Parser, Debug)]
pub struct Login {}

impl Command for Login {
    async fn run(&self, _args: &Args) {
        print!("username: ");
        io::stdout().flush().unwrap();
        input! {username: String};
        let password = rpassword::prompt_password("password: ").unwrap();
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
