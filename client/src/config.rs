use std::{collections::HashMap, process::exit};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    pub(crate) default: DefaultConfig,
    pub(crate) servers: HashMap<String, ServerConfig>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct DefaultConfig {
    pub(crate) server: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ServerConfig {
    pub(crate) url: url::Url,
    pub(crate) token: Option<String>,
}

impl Config {
    pub(crate) fn load() -> Self {
        confy::load("freight-train", "config").unwrap_or_else(|error| {
            eprintln!("{:?}", error);
            exit(1);
        })
    }

    pub(crate) fn store(&self) {
        confy::store("freight-train", "config", self).unwrap_or_else(|error| eprintln!("{error}"))
    }
}
