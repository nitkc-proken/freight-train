use std::sync::{LazyLock, Mutex};

use openapi::apis::configuration::Configuration;


pub fn get_api_config(basepath:String) -> Configuration {
    let mut conf = Configuration::new();
    conf.base_path = basepath;
    conf
}