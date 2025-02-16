#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponse {
    pub ok: bool,
    pub data: Option<LoginResponseData>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponseData {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    pub username: String,
    pub token: LoginResponseToken,
}

#[derive(Debug, Deserialize)]
pub struct LoginResponseToken {
    pub token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
pub struct LogoutResponse {
    pub ok: bool,
    pub data: Option<String>, // Always returned null or empty
    pub message: Option<String>,
}
