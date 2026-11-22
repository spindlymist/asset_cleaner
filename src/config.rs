use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &'static str = "asset_cleaner.toml";

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AppConfig {
    pub ks_path: Option<PathBuf>,
}

pub fn load_config() -> Result<AppConfig> {
    let contents = fs::read_to_string(CONFIG_FILE)?;
    let config: AppConfig = toml::de::from_str(&contents)?;
    
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let contents = toml::ser::to_string(config)?;
    fs::write(CONFIG_FILE, contents)?;
    
    Ok(())
}
