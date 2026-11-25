use must_config::schema::{Config, EnvValue};
use std::collections::HashMap;

/// Compose the environment for a recipe execution.
///
/// Layers are applied in this order and each overrides the ones before it:
///   process env → global [env] → profile [env.<profile>] → recipe env → toolchain env
///
/// A `${NAME}` reference in a config value is replaced with the value NAME has
/// in the layers beneath it; unknown references are left as written.
pub fn compose_env(
    config: &Config,
    recipe_name: &str,
    profile: &str,
    toolchain_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let base: HashMap<String, String> = std::env::vars().collect();
    compose_env_with_base(config, recipe_name, profile, toolchain_env, &base)
}

fn is_env_name(name: &str) -> bool {
    !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn expand(value: &str, env: &HashMap<String, String>) -> String {
    if !value.contains("${") {
        return value.to_string();
    }
    let mut out = String::with_capacity(value.len());
    let mut parts = value.split("${");
    out.push_str(parts.next().unwrap_or_default());
    for part in parts {
        match part.split_once('}') {
            Some((name, tail)) if is_env_name(name) => match env.get(name) {
                Some(replacement) => {
                    out.push_str(replacement);
                    out.push_str(tail);
                }
                None => {
                    out.push_str("${");
                    out.push_str(part);
                }
            },
            _ => {
                out.push_str("${");
                out.push_str(part);
            }
        }
    }
    out
}

pub fn compose_env_with_base(
    config: &Config,
    recipe_name: &str,
    profile: &str,
    toolchain_env: &HashMap<String, String>,
    base_env: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut env: HashMap<String, String> = base_env.clone();

    // Layer 2: global [env] scalars
    let base = env.clone();
    for (key, val) in &config.env.global {
        if let EnvValue::Scalar(s) = val {
            env.insert(key.clone(), expand(s, &base));
        }
    }

    // Layer 3: profile [env.<profile>] (stored as EnvValue::Profile maps)
    let lower = env.clone();
    for (key, val) in &config.env.global {
        if let EnvValue::Profile(profile_map) = val
            && key == profile
        {
            for (k, v) in profile_map {
                env.insert(k.clone(), expand(v, &lower));
            }
        }
    }

    // Layer 4: per-recipe env
    let lower = env.clone();
    if let Some(recipe) = config.recipe.get(recipe_name) {
        for (k, v) in &recipe.env {
            env.insert(k.clone(), expand(v, &lower));
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
    fn test_env_values_reference_lower_layers() {
        let cfg: Config = toml::from_str(
            r#"
[project]
name = "test"

[env]
ROOT = "${HOME}/srv"
LOG = "info"

[env.release]
LOG = "warn-${ROOT}"

[recipe.build]
type = "shell"
script = "true"

[recipe.build.env]
CACHE = "${ROOT}/cache"
MUST_ARGS = "${MUST_ARGS}"
UNSET_REF = "${NOT_DEFINED_ANYWHERE}"
"#,
        )
        .unwrap();
        let mut base = HashMap::new();
        base.insert("HOME".to_string(), "/home/dev".to_string());
        base.insert("MUST_ARGS".to_string(), "hello".to_string());
        let env = compose_env_with_base(&cfg, "build", "release", &HashMap::new(), &base);
        assert_eq!(env["ROOT"], "/home/dev/srv");
        assert_eq!(env["LOG"], "warn-/home/dev/srv");
        assert_eq!(env["CACHE"], "/home/dev/srv/cache");
        assert_eq!(env["MUST_ARGS"], "hello");
        assert_eq!(env["UNSET_REF"], "${NOT_DEFINED_ANYWHERE}");
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
