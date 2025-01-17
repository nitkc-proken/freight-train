use std::collections::HashMap;

use serde_derive::{Serialize, Deserialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) struct Config {
    servers: HashMap<String, String>,
    token: Option<String>
}

impl Config {
    pub(crate) fn load() -> Self {
        confy::load("freight-train", "config").unwrap()
    }

    #[allow(dead_code)] // 使用時はこのアノテーションを取り除くこと
    pub(crate) fn store(&self) {
        confy::store("freight-train", "config", self).unwrap_or_else(|error| eprintln!("{error}"))
    }
}
