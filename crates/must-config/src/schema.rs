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
    pub scripts: HashMap<String, String>,
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
        let os = std::env::consts::OS;
        let family = if cfg!(unix) { "unix" } else if cfg!(windows) { "windows" } else { "" };

        let candidates: Vec<&str> = match os {
            "linux" => vec![os, family],
            "macos" => vec![os, family],
            "freebsd" | "netbsd" | "openbsd" => vec![os, "bsd", family],
            "windows" => vec!["win", os, family],
            _ => vec![os, family],
        };

        for key in &candidates {
            if let Some(script) = self.scripts.get(*key) {
                return Some(script);
            }
        }

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

    fn test_recipe() -> Recipe {
        Recipe {
            recipe_type: RecipeType::Shell,
            script: None,
            script_win: None,
            scripts: HashMap::new(),
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
        }
    }

    #[test]
    fn test_resolved_script_no_override() {
        let mut r = test_recipe();
        r.script = Some("echo default".into());
        assert_eq!(r.resolved_script().unwrap(), "echo default");
    }

    #[test]
    fn test_resolved_script_with_win_override() {
        let mut r = test_recipe();
        r.script = Some("rm -rf build".into());
        r.script_win = Some("rmdir /s /q build".into());
        if cfg!(windows) {
            assert_eq!(r.resolved_script().unwrap(), "rmdir /s /q build");
        } else {
            assert_eq!(r.resolved_script().unwrap(), "rm -rf build");
        }
    }

    #[test]
    fn test_resolved_script_win_only() {
        let mut r = test_recipe();
        r.script_win = Some("dir".into());
        if cfg!(windows) {
            assert_eq!(r.resolved_script().unwrap(), "dir");
        } else {
            assert_eq!(r.resolved_script(), None);
        }
    }

    #[test]
    fn test_resolved_script_scripts_map_takes_priority() {
        let mut r = test_recipe();
        r.script = Some("default".into());
        let os = std::env::consts::OS.to_string();
        r.scripts.insert(os.clone(), "from-scripts-map".into());
        assert_eq!(r.resolved_script().unwrap(), "from-scripts-map");
    }

    #[test]
    fn test_resolved_script_scripts_map_unix_fallback() {
        let mut r = test_recipe();
        r.script = Some("default".into());
        r.scripts.insert("unix".into(), "unix-script".into());
        if cfg!(unix) {
            assert_eq!(r.resolved_script().unwrap(), "unix-script");
        } else {
            assert_eq!(r.resolved_script().unwrap(), "default");
        }
    }

    #[test]
    fn test_resolved_script_scripts_map_bsd_fallback() {
        let mut r = test_recipe();
        r.script = Some("default".into());
        r.scripts.insert("bsd".into(), "bsd-script".into());
        if cfg!(target_os = "freebsd") || cfg!(target_os = "netbsd") || cfg!(target_os = "openbsd") {
            assert_eq!(r.resolved_script().unwrap(), "bsd-script");
        } else {
            assert_eq!(r.resolved_script().unwrap(), "default");
        }
    }

    #[test]
    fn test_resolved_script_scripts_map_os_beats_unix() {
        let mut r = test_recipe();
        r.script = Some("default".into());
        r.scripts.insert("unix".into(), "unix".into());
        let os = std::env::consts::OS.to_string();
        r.scripts.insert(os, "os-specific".into());
        assert_eq!(r.resolved_script().unwrap(), "os-specific");
    }

    #[test]
    fn test_resolved_script_scripts_map_beats_script_win() {
        let mut r = test_recipe();
        r.script = Some("default".into());
        r.script_win = Some("win-fallback".into());
        r.scripts.insert("win".into(), "win-from-scripts".into());
        if cfg!(windows) {
            assert_eq!(r.resolved_script().unwrap(), "win-from-scripts");
        } else {
            assert_eq!(r.resolved_script().unwrap(), "default");
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

    #[test]
    fn test_scripts_map_parsed_from_toml() {
        let toml = r#"
[project]
name = "test"

[recipe.build]
type   = "shell"
script = "make"

[recipe.build.scripts]
macos   = "make -j$(sysctl -n hw.ncpu)"
linux   = "make -j$(nproc)"
win     = "nmake"
freebsd = "gmake"
"#;
        let config: Config = toml::from_str(toml).unwrap();
        let recipe = &config.recipe["build"];
        assert_eq!(recipe.script.as_deref(), Some("make"));
        assert_eq!(recipe.scripts.get("macos").unwrap(), "make -j$(sysctl -n hw.ncpu)");
        assert_eq!(recipe.scripts.get("linux").unwrap(), "make -j$(nproc)");
        assert_eq!(recipe.scripts.get("win").unwrap(), "nmake");
        assert_eq!(recipe.scripts.get("freebsd").unwrap(), "gmake");
    }
}
