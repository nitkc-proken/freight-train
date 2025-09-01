use std::sync::{Arc, LazyLock, Mutex};

use common::protocol::Protocol;
use config::Config;
use garde::Validate;
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Validate, Debug)]
pub struct ServerConfig {
    #[garde(dive)]
    pub tunnel: TunnelServiceConfig,
    #[garde(dive)]
    pub grpc: GrpcServiceConfig,
    #[garde(url, prefix("sqlite://"))]
    pub sqlite_url: String,
    #[garde(url, pattern("^https?:\\/\\/.*$"))]
    pub backend_grpc_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct TunnelServiceConfig {
    #[garde(length(min = 1), ip)]
    pub host: String,
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,
    #[garde(skip)]
    pub protocol: Protocol,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct GrpcServiceConfig {
    #[garde(length(min = 1))]
    pub host: String,
    #[garde(range(min = 1, max = 65535))]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tunnel: TunnelServiceConfig {
                host: "localhost".to_owned(),
                port: 8080,
                protocol: Protocol::Quic,
            },
            grpc: GrpcServiceConfig {
                host: "localhost".to_owned(),
                port: 50051,
            },
            backend_grpc_url: "http://localhost:50051".to_owned(),
            sqlite_url: "sqlite://db.sqlite?mode=rwc".to_owned(),
        }
    }
}

pub static SERVER_CONFIG: LazyLock<Arc<ServerConfig>> = LazyLock::new(|| {
    let config = ServerConfig::load_config().unwrap();
    Arc::new(config)
});

const CONFIG_FILE_NAME: &str = "config.toml";

impl ServerConfig {
    fn load_config() -> Result<Self, String> {
        if !std::path::Path::new(CONFIG_FILE_NAME).exists() {
            // save defaults to config file
            let default_config = ServerConfig::default();
            let toml_string = toml::to_string(&default_config)
                .map_err(|e| format!("Failed to serialize default config: {}", e))?;
            std::fs::write(CONFIG_FILE_NAME, toml_string)
                .map_err(|e| format!("Failed to write default config to file: {}", e))?;
        }

        let config = Config::builder()
            .add_source(config::File::with_name(CONFIG_FILE_NAME))
            .add_source(config::Environment::with_prefix("FR8"))
            .build()
            .unwrap();
        let server_config = config
            .try_deserialize::<Self>()
            .map_err(|e| "Failed to load config: ".to_owned() + &e.to_string())?;
        server_config.validate().map_err(|e| e.to_string())?;
        Ok(server_config)
    }
}
