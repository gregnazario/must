use crate::schema::Config;
use must_core::Error;
use std::collections::HashSet;
use std::path::Path;

pub fn validate(config: &Config, path: &Path) -> must_core::Result<()> {
    let recipe_names: HashSet<&str> = config.recipe.keys().map(|s| s.as_str()).collect();

    for (name, recipe) in &config.recipe {
        for dep in &recipe.deps {
            if !recipe_names.contains(dep.as_str()) {
                return Err(Error::Config {
                    path: path.to_owned(),
                    message: format!("recipe '{name}' depends on '{dep}' which does not exist"),
                });
            }
        }
    }

    Ok(())
}
