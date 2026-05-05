use crate::schema::{Config, RecipeType};
use must_core::Error;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

        let missing = match &recipe.recipe_type {
            RecipeType::Shell => require_field("script", recipe.script.as_ref(), name),
            RecipeType::RustBin | RecipeType::RustLib | RecipeType::RustTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::GoBin | RecipeType::GoTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::CBin | RecipeType::CLib => {
                require_non_empty_vec("sources", &recipe.sources, name)
            }
            RecipeType::TsBin | RecipeType::TsCheck | RecipeType::TsLint => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::Npm => require_field("script", recipe.script.as_ref(), name),
            RecipeType::PyBin | RecipeType::PyTest | RecipeType::PyLint => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::ZigBin | RecipeType::ZigTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::DockerBuild | RecipeType::DockerPush => {
                require_field("image", recipe.image.as_ref(), name)
            }
            RecipeType::Plugin => {
                require_field("plugin", recipe.plugin.as_ref(), name)
            }
            RecipeType::JavaBin | RecipeType::JavaTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::KotlinBin | RecipeType::KotlinTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
            RecipeType::SwiftBin | RecipeType::SwiftTest => {
                require_field("package", recipe.package.as_ref(), name)
            }
        };

        if let Some(err) = missing {
            return Err(err);
        }
    }

    Ok(())
}

fn require_field(field: &str, value: Option<&String>, recipe_name: &str) -> Option<Error> {
    if value.is_none() || value.is_none_or(|v| v.is_empty()) {
        Some(Error::Config {
            path: PathBuf::new(),
            message: format!("recipe '{recipe_name}' is missing required field '{field}'"),
        })
    } else {
        None
    }
}

fn require_non_empty_vec(field: &str, vec: &[String], recipe_name: &str) -> Option<Error> {
    if vec.is_empty() {
        Some(Error::Config {
            path: PathBuf::new(),
            message: format!("recipe '{recipe_name}' is missing required field '{field}'"),
        })
    } else {
        None
    }
}
