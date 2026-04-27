use must_config::schema::{Config, EnvValue};
use std::collections::HashMap;

/// Compose the environment for a recipe execution.
///
/// Priority order (highest wins):
///   process env → global [env] → profile [env.<profile>] → recipe env → toolchain env
pub fn compose_env(
    config: &Config,
    recipe_name: &str,
    profile: &str,
    toolchain_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = std::env::vars().collect();

    // Layer 2: global [env] scalars
    for (key, val) in &config.env.global {
        if let EnvValue::Scalar(s) = val {
            env.insert(key.clone(), s.clone());
        }
    }

    // Layer 3: profile [env.<profile>] (stored as EnvValue::Profile maps)
    for (key, val) in &config.env.global {
        if let EnvValue::Profile(profile_map) = val {
            if key == profile {
                for (k, v) in profile_map {
                    env.insert(k.clone(), v.clone());
                }
            }
        }
    }

    // Layer 4: per-recipe env
    if let Some(recipe) = config.recipe.get(recipe_name) {
        for (k, v) in &recipe.env {
            env.insert(k.clone(), v.clone());
        }
    }

    // Layer 5: toolchain env (highest priority)
    for (k, v) in toolchain_env {
        env.insert(k.clone(), v.clone());
    }

    env
}
