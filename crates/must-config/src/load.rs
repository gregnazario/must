use crate::schema::{Config, IncludeFragment};
use crate::validate;
use must_core::Error;
use std::path::Path;

pub fn load_config(path: &Path) -> must_core::Result<Config> {
    let content = std::fs::read_to_string(path).map_err(|e| Error::Config {
        path: path.to_owned(),
        message: format!("could not read file: {e}"),
    })?;
    let mut config: Config = toml::from_str(&content).map_err(|e| Error::Config {
        path: path.to_owned(),
        message: e.to_string(),
    })?;

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));

    for include_path in &config.project.include {
        let full_path = base_dir.join(include_path);
        let inc_content = std::fs::read_to_string(&full_path).map_err(|e| Error::Config {
            path: full_path.clone(),
            message: format!("could not read include: {e}"),
        })?;
        let fragment: IncludeFragment =
            toml::from_str(&inc_content).map_err(|e| Error::Config {
                path: full_path.clone(),
                message: format!("invalid include: {e}"),
            })?;

        for (k, v) in fragment.env.global {
            config.env.global.entry(k).or_insert(v);
        }
        for (k, v) in fragment.targets {
            config.targets.entry(k).or_insert(v);
        }
        for (k, v) in fragment.recipe {
            config.recipe.entry(k).or_insert(v);
        }
    }

    validate::validate(&config, path)?;
    Ok(config)
}
