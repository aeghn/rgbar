use std::{collections::HashMap, env, path::Path, sync::Arc};

use arc_swap::ArcSwap;
use chin_tools::wrapper::anyhow::AResult;
use serde::Deserialize;

lazy_static::lazy_static! {
    static ref CONFIG: ArcSwap<Option<Config>> = ArcSwap::new(Arc::new(None));
}

pub fn get_config() -> Arc<Option<Config>> {
    CONFIG.load().clone()
}

pub fn set_config(config: Config) {
    CONFIG.store(Arc::new(Some(config)));
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub paths: Vec<String>,
    pub alias: HashMap<String, Vec<String>>,
}

impl Config {
    pub fn read_from_json_file<T: AsRef<Path>>(filepath: Option<T>) -> AResult<Self> {
        let default_config_dir = match env::var("XDG_CONFIG_PATH") {
            Ok(path) => path,
            Err(_) => format!("{}/.config", env::var("HOME").unwrap()),
        };

        let config_filepath = format!("{}/rglauncher/rgbar.json", default_config_dir);
        let config_filepath = match filepath.as_ref() {
            Some(path) => path.as_ref(),
            None => config_filepath.as_ref(),
        };
        let config_str = std::fs::read_to_string(config_filepath).expect(&format!(
            "Unable to read config content. {:?}",
            config_filepath
        ));

        Ok(serde_json::from_str(&config_str.as_str())?)
    }
}
