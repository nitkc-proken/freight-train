use std::sync::{Arc, LazyLock};

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use crate::config::SERVER_CONFIG;

pub async fn get_client() -> DatabaseConnection {
    let db_url = SERVER_CONFIG.sqlite_url.clone();
    let opt = ConnectOptions::new(db_url);
    Database::connect(opt).await.unwrap()
}
