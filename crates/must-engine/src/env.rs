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
        if let EnvValue::Profile(profile_map) = val
            && key == profile
        {
            for (k, v) in profile_map {
                env.insert(k.clone(), v.clone());
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

#[cfg(test)]
mod tests {
    use super::*;
    use must_config::schema::Config;
    use std::collections::HashMap;

    fn minimal_config(name: &str) -> Config {
        toml::from_str(&format!(
            r#"
[project]
name = "{name}"
"#
        ))
        .unwrap()
    }

    #[test]
    fn test_global_env_applied() {
        let cfg: Config = toml::from_str(
            r#"
[project]
name = "test"

[env]
FOO = "bar"
"#,
        )
        .unwrap();
        let env = compose_env(&cfg, "build", "default", &HashMap::new());
        assert_eq!(env.get("FOO").map(String::as_str), Some("bar"));
    }

    #[test]
    fn test_profile_env_overrides_global() {
        let cfg: Config = toml::from_str(
            r#"
[project]
name = "test"

[env]
LOG = "info"

[env.release]
LOG = "warn"
"#,
        )
        .unwrap();
        let dev_env = compose_env(&cfg, "build", "default", &HashMap::new());
        assert_eq!(dev_env.get("LOG").map(String::as_str), Some("info"));

        let rel_env = compose_env(&cfg, "build", "release", &HashMap::new());
        assert_eq!(rel_env.get("LOG").map(String::as_str), Some("warn"));
    }

    #[test]
    fn test_recipe_env_overrides_global() {
        let cfg: Config = toml::from_str(
            r#"
[project]
name = "test"

[env]
DEBUG = "0"

[recipe.build]
type = "shell"
script = "echo hi"

[recipe.build.env]
DEBUG = "1"
"#,
        )
        .unwrap();
        let env = compose_env(&cfg, "build", "default", &HashMap::new());
        assert_eq!(env.get("DEBUG").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_toolchain_env_has_highest_priority() {
        let cfg: Config = toml::from_str(
            r#"
[project]
name = "test"

[env]
CC = "gcc"

[recipe.build]
type = "shell"
script = "echo hi"

[recipe.build.env]
CC = "clang"
"#,
        )
        .unwrap();
        let toolchain = HashMap::from([("CC".to_string(), "aarch64-linux-gnu-gcc".to_string())]);
        let env = compose_env(&cfg, "build", "default", &toolchain);
        assert_eq!(
            env.get("CC").map(String::as_str),
            Some("aarch64-linux-gnu-gcc")
        );
    }

    #[test]
    fn test_unknown_recipe_name_returns_base_env() {
        let cfg = minimal_config("test");
        let env = compose_env(&cfg, "nonexistent", "default", &HashMap::new());
        // Should not panic, just return env without recipe layer
        assert!(!env.is_empty()); // process env is always present
    }
}
