use std::{fs::{self, File}, io::Write, path::PathBuf};

use anyhow::Result;

const CONFIG_FILE: &'static str = "asset_cleaner.ini";

#[derive(Debug, Default)]
pub struct AppConfig {
    pub ks_path: Option<PathBuf>,
}

pub fn load_config() -> Result<AppConfig> {
    let ini_contents = fs::read_to_string(CONFIG_FILE)?;
    let ini = libks_ini::Ini::new(&ini_contents);
    
    let ks_path = ini.get_in("Paths", "ks")
        .map(PathBuf::from);
    
    Ok(AppConfig {
        ks_path,
    })
}

pub fn save_config(config: &AppConfig) -> Result<()> {
    let mut file = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(CONFIG_FILE)?;
    let mut ini = libks_ini::Ini::new("");
    if let Some(path) = config.ks_path.as_ref() {
        ini.set_in("Paths", "ks", path.to_string_lossy().into_owned());
    }
    write!(file, "{}", ini.to_string())?;
    
    Ok(())
}
