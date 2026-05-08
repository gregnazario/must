use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level Mustfile.toml configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub project: Project,
    #[serde(default)]
    pub env: EnvMap,
    #[serde(default)]
    pub targets: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub recipe: HashMap<String, Recipe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Project {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

/// Env vars table. Supports both flat `[env]` and profile-scoped `[env.release]`.
/// Since TOML can't have both string values and subtable in one table,
/// we store it as a flat map and profile maps separately.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EnvMap {
    #[serde(flatten)]
    pub global: HashMap<String, EnvValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EnvValue {
    Scalar(String),
    Profile(HashMap<String, String>),
}

/// A recipe definition parsed from Mustfile.toml.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Recipe {
    #[serde(rename = "type")]
    pub recipe_type: RecipeType,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub script_win: Option<String>,
    #[serde(default)]
    pub cache: Option<CacheMode>,
    #[serde(default)]
    pub phony: bool,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub cross: HashMap<String, CrossConfig>,
    // rust-bin / rust-lib / rust-test fields
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    // go-bin / go-test fields
    #[serde(default)]
    pub ldflags: Option<String>,
    // c-bin / c-lib fields
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub link_libs: Vec<String>,
    // docker-build / docker-push fields
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub dockerfile: Option<String>,
    #[serde(default)]
    pub build_args: Vec<String>,
    // plugin fields
    #[serde(default)]
    pub plugin: Option<String>,
    // precompiled-bin fields
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
}

impl Recipe {
    pub fn resolved_script(&self) -> Option<&String> {
        if cfg!(windows) {
            self.script_win.as_ref().or(self.script.as_ref())
        } else {
            self.script.as_ref()
        }
    }
}

/// Supported recipe type identifiers. Each maps to a language-specific build tool.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeType {
    Shell,
    RustBin,
    RustLib,
    RustTest,
    GoBin,
    GoTest,
    CBin,
    CLib,
    TsBin,
    TsCheck,
    TsLint,
    Npm,
    PyBin,
    PyTest,
    PyLint,
    ZigBin,
    ZigTest,
    DockerBuild,
    DockerPush,
    Plugin,
    JavaBin,
    JavaTest,
    KotlinBin,
    KotlinTest,
    SwiftBin,
    SwiftTest,
    DotnetBuild,
    DotnetTest,
    DotnetPublish,
    RubyBin,
    RubyTest,
    DartBin,
    DartTest,
    ElixirBuild,
    ElixirTest,
    FlutterBuild,
    FlutterTest,
    NimBin,
    NimTest,
    PrecompiledBin,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CacheMode {
    Hash,
    Mtime,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrossConfig {
    #[serde(default)]
    pub linker: Option<String>,
    #[serde(default)]
    pub cross: Option<CrossBackend>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct IncludeFragment {
    #[serde(default)]
    pub env: EnvMap,
    #[serde(default)]
    pub targets: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub recipe: HashMap<String, Recipe>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CrossBackend {
    Container,
    Local,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_script_no_override() {
        let r = Recipe {
            recipe_type: RecipeType::Shell,
            script: Some("echo unix".into()),
            script_win: None,
            deps: vec![],
            inputs: vec![],
            outputs: vec![],
            cache: None,
            phony: false,
            env: HashMap::new(),
            cross: HashMap::new(),
            package: None,
            features: vec![],
            ldflags: None,
            sources: vec![],
            includes: vec![],
            link_libs: vec![],
            image: None,
            dockerfile: None,
            build_args: vec![],
            plugin: None,
            url: None,
            sha256: None,
        };
        let resolved = r.resolved_script().unwrap();
        assert_eq!(resolved, "echo unix");
    }

    #[test]
    fn test_resolved_script_with_win_override() {
        let r = Recipe {
            recipe_type: RecipeType::Shell,
            script: Some("rm -rf build".into()),
            script_win: Some("rmdir /s /q build".into()),
            deps: vec![],
            inputs: vec![],
            outputs: vec![],
            cache: None,
            phony: false,
            env: HashMap::new(),
            cross: HashMap::new(),
            package: None,
            features: vec![],
            ldflags: None,
            sources: vec![],
            includes: vec![],
            link_libs: vec![],
            image: None,
            dockerfile: None,
            build_args: vec![],
            plugin: None,
            url: None,
            sha256: None,
        };
        if cfg!(windows) {
            assert_eq!(r.resolved_script().unwrap(), "rmdir /s /q build");
        } else {
            assert_eq!(r.resolved_script().unwrap(), "rm -rf build");
        }
    }

    #[test]
    fn test_resolved_script_win_only() {
        let r = Recipe {
            recipe_type: RecipeType::Shell,
            script: None,
            script_win: Some("dir".into()),
            deps: vec![],
            inputs: vec![],
            outputs: vec![],
            cache: None,
            phony: false,
            env: HashMap::new(),
            cross: HashMap::new(),
            package: None,
            features: vec![],
            ldflags: None,
            sources: vec![],
            includes: vec![],
            link_libs: vec![],
            image: None,
            dockerfile: None,
            build_args: vec![],
            plugin: None,
            url: None,
            sha256: None,
        };
        if cfg!(windows) {
            assert_eq!(r.resolved_script().unwrap(), "dir");
        } else {
            assert_eq!(r.resolved_script(), None);
        }
    }

    #[test]
    fn test_script_win_parsed_from_toml() {
        let toml = r#"
[project]
name = "test"

[recipe.clean]
type       = "shell"
script     = "rm -rf build"
script_win = "rmdir /s /q build"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let recipe = &config.recipe["clean"];
        assert_eq!(recipe.script.as_deref(), Some("rm -rf build"));
        assert_eq!(recipe.script_win.as_deref(), Some("rmdir /s /q build"));
    }
}
