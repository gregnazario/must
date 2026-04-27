use crate::schema::Config;
use crate::validate;
use must_core::Error;
use std::path::Path;

pub fn load_config(path: &Path) -> must_core::Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Config {
        path: path.to_owned(),
        message: format!("could not read file: {e}"),
    })?;
    let config: Config = toml::from_str(&content).map_err(|e| Error::Config {
        path: path.to_owned(),
        message: e.to_string(),
    })?;
    validate::validate(&config, path)?;
    Ok(config)
}
