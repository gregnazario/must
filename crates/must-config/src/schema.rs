use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
}

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
