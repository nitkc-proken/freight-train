use sea_orm::DatabaseConnection;

use crate::db::get_client;

pub struct ServerState {
    pub db_conn: DatabaseConnection,
}

impl ServerState {
    pub async fn new() -> Self {
        let db_conn = get_client().await;
        Self { db_conn }
    }
}
