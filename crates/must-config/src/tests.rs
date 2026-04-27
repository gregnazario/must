use crate::schema::{CacheMode, CrossBackend, EnvValue, RecipeType};
use crate::{load_config, validate};
use std::path::Path;

fn parse(toml: &str) -> crate::schema::Config {
    toml::from_str(toml).expect("valid TOML")
}

fn parse_err(toml: &str) -> String {
    toml::from_str::<crate::schema::Config>(toml)
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default()
}

// ── Minimal config ────────────────────────────────────────────────────────────

#[test]
fn test_minimal_config() {
    let cfg = parse(
        r#"
[project]
name = "myapp"
"#,
    );
    assert_eq!(cfg.project.name, "myapp");
    assert!(cfg.project.version.is_none());
    assert!(cfg.recipe.is_empty());
    assert!(cfg.targets.is_empty());
}

#[test]
fn test_project_with_version() {
    let cfg = parse(
        r#"
[project]
name = "myapp"
version = "1.2.3"
"#,
    );
    assert_eq!(cfg.project.version.as_deref(), Some("1.2.3"));
}

// ── Shell recipe ──────────────────────────────────────────────────────────────

#[test]
fn test_shell_recipe_minimal() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo hello"
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::Shell);
    assert_eq!(r.script.as_deref(), Some("echo hello"));
    assert!(r.deps.is_empty());
    assert!(!r.phony);
}

#[test]
fn test_shell_recipe_full() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.codegen]
type = "shell"
inputs = ["proto/**/*.proto"]
outputs = ["src/generated/**/*.rs"]
cache = "hash"
phony = false
script = "protoc --rust_out=src/generated proto/*.proto"

[recipe.clean]
type = "shell"
phony = true
script = "rm -rf dist/"
"#,
    );
    let codegen = &cfg.recipe["codegen"];
    assert_eq!(codegen.inputs, ["proto/**/*.proto"]);
    assert_eq!(codegen.outputs, ["src/generated/**/*.rs"]);
    assert_eq!(codegen.cache, Some(CacheMode::Hash));

    let clean = &cfg.recipe["clean"];
    assert!(clean.phony);
}

#[test]
fn test_recipe_with_deps() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.codegen]
type = "shell"
script = "echo codegen"

[recipe.build]
type = "shell"
deps = ["codegen"]
script = "echo build"

[recipe.release]
type = "shell"
deps = ["build", "codegen"]
script = "echo release"
"#,
    );
    assert_eq!(cfg.recipe["build"].deps, ["codegen"]);
    assert_eq!(cfg.recipe["release"].deps, ["build", "codegen"]);
}

#[test]
fn test_recipe_env_overrides() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "cargo build"

[recipe.build.env]
RUST_LOG = "debug"
CARGO_PROFILE = "dev"
"#,
    );
    let env = &cfg.recipe["build"].env;
    assert_eq!(env["RUST_LOG"], "debug");
    assert_eq!(env["CARGO_PROFILE"], "dev");
}

// ── Env tables ────────────────────────────────────────────────────────────────

#[test]
fn test_global_env_scalars() {
    let cfg = parse(
        r#"
[project]
name = "test"

[env]
RUST_LOG = "info"
DATABASE_URL = "postgres://localhost/mydb"
"#,
    );
    let global = &cfg.env.global;
    assert!(matches!(&global["RUST_LOG"], EnvValue::Scalar(s) if s == "info"));
    assert!(
        matches!(&global["DATABASE_URL"], EnvValue::Scalar(s) if s == "postgres://localhost/mydb")
    );
}

#[test]
fn test_profile_env() {
    let cfg = parse(
        r#"
[project]
name = "test"

[env]
RUST_LOG = "info"

[env.release]
RUST_LOG = "warn"
OPTIMIZE = "true"
"#,
    );
    // Global scalar
    assert!(matches!(&cfg.env.global["RUST_LOG"], EnvValue::Scalar(s) if s == "info"));
    // Profile map stored under "release" key
    assert!(matches!(&cfg.env.global["release"], EnvValue::Profile(m) if m["RUST_LOG"] == "warn"));
    if let EnvValue::Profile(m) = &cfg.env.global["release"] {
        assert_eq!(m["OPTIMIZE"], "true");
    }
}

// ── Targets ───────────────────────────────────────────────────────────────────

#[test]
fn test_targets() {
    let cfg = parse(
        r#"
[project]
name = "test"

[targets]
default = ["x86_64-linux-gnu"]
release = ["x86_64-linux-gnu", "aarch64-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"]
"#,
    );
    assert_eq!(cfg.targets["default"], ["x86_64-linux-gnu"]);
    assert_eq!(cfg.targets["release"].len(), 4);
    assert!(cfg.targets["release"].contains(&"aarch64-apple-darwin".to_string()));
}

// ── Language recipe types ─────────────────────────────────────────────────────

#[test]
fn test_rust_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "myapp"
features = ["cli", "tls"]
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::RustBin);
    assert_eq!(r.package.as_deref(), Some("myapp"));
    assert_eq!(r.features, ["cli", "tls"]);
}

#[test]
fn test_rust_lib_and_test_recipes() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.lib]
type = "rust-lib"
package = "mylib"

[recipe.test]
type = "rust-test"
package = "myapp"
"#,
    );
    assert_eq!(cfg.recipe["lib"].recipe_type, RecipeType::RustLib);
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::RustTest);
}

#[test]
fn test_go_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.serve]
type = "go-bin"
package = "./cmd/server"
ldflags = "-s -w"
"#,
    );
    let r = &cfg.recipe["serve"];
    assert_eq!(r.recipe_type, RecipeType::GoBin);
    assert_eq!(r.package.as_deref(), Some("./cmd/server"));
    assert_eq!(r.ldflags.as_deref(), Some("-s -w"));
}

#[test]
fn test_go_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "go-test"
package = "./..."
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::GoTest);
}

#[test]
fn test_c_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "c-bin"
sources = ["src/main.c", "src/util.c"]
includes = ["include/"]
link_libs = ["m", "pthread"]
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::CBin);
    assert_eq!(r.sources, ["src/main.c", "src/util.c"]);
    assert_eq!(r.includes, ["include/"]);
    assert_eq!(r.link_libs, ["m", "pthread"]);
}

#[test]
fn test_c_lib_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.libfoo]
type = "c-lib"
sources = ["src/foo.c"]
"#,
    );
    assert_eq!(cfg.recipe["libfoo"].recipe_type, RecipeType::CLib);
}

// ── Cross-compile config ──────────────────────────────────────────────────────

#[test]
fn test_cross_config_local() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "myapp"

[recipe.build.cross]
"aarch64-linux-gnu" = { linker = "aarch64-linux-gnu-gcc" }
"#,
    );
    let cross = &cfg.recipe["build"].cross;
    let aarch = &cross["aarch64-linux-gnu"];
    assert_eq!(aarch.linker.as_deref(), Some("aarch64-linux-gnu-gcc"));
    assert!(aarch.cross.is_none());
}

#[test]
fn test_cross_config_container() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "myapp"

[recipe.build.cross]
"aarch64-linux-gnu" = { linker = "aarch64-linux-gnu-gcc", cross = "container" }
"#,
    );
    let aarch = &cfg.recipe["build"].cross["aarch64-linux-gnu"];
    assert_eq!(aarch.cross, Some(CrossBackend::Container));
}

// ── Cache modes ───────────────────────────────────────────────────────────────

#[test]
fn test_cache_modes() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.hashed]
type = "shell"
script = "echo hash"
cache = "hash"

[recipe.mtimed]
type = "shell"
script = "echo mtime"
cache = "mtime"

[recipe.uncached]
type = "shell"
script = "echo none"
cache = "none"
"#,
    );
    assert_eq!(cfg.recipe["hashed"].cache, Some(CacheMode::Hash));
    assert_eq!(cfg.recipe["mtimed"].cache, Some(CacheMode::Mtime));
    assert_eq!(cfg.recipe["uncached"].cache, Some(CacheMode::None));
    assert!(cfg
        .recipe
        .get("hashed")
        .map(|r| r.cache.is_some())
        .unwrap_or(false));
}

// ── Validation ────────────────────────────────────────────────────────────────

#[test]
fn test_validation_passes_for_valid_deps() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.a]
type = "shell"
script = "echo a"

[recipe.b]
type = "shell"
deps = ["a"]
script = "echo b"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_ok());
}

#[test]
fn test_validation_fails_for_unknown_dep() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
deps = ["nonexistent"]
script = "echo build"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nonexistent"),
        "error should name the missing dep: {msg}"
    );
}

#[test]
fn test_validation_fails_for_chain_with_missing_link() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.a]
type = "shell"
script = "echo a"

[recipe.c]
type = "shell"
deps = ["b"]
script = "echo c"
"#,
    );
    // 'b' is referenced but not defined
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
}

// ── load_config round-trip ────────────────────────────────────────────────────

#[test]
fn test_load_config_from_file() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
[project]
name = "roundtrip"
version = "0.1.0"

[env]
RUST_LOG = "info"

[recipe.build]
type = "rust-bin"
package = "roundtrip"
deps = []
"#
    )
    .unwrap();
    let cfg = load_config(f.path()).unwrap();
    assert_eq!(cfg.project.name, "roundtrip");
    assert_eq!(cfg.recipe["build"].recipe_type, RecipeType::RustBin);
}

#[test]
fn test_load_config_missing_file() {
    let result = load_config(Path::new("/no/such/Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("could not read file"),
        "expected read error, got: {msg}"
    );
}

#[test]
fn test_load_config_invalid_toml() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(f, "this is not valid toml ][").unwrap();
    let result = load_config(f.path());
    assert!(result.is_err());
}

#[test]
fn test_load_config_validates_deps() {
    use std::io::Write;
    let mut f = tempfile::NamedTempFile::new().unwrap();
    write!(
        f,
        r#"
[project]
name = "bad"

[recipe.build]
type = "shell"
deps = ["ghost"]
script = "echo build"
"#
    )
    .unwrap();
    let result = load_config(f.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("ghost"),
        "should name missing dep in error: {msg}"
    );
}

// ── Parse errors ─────────────────────────────────────────────────────────────

#[test]
fn test_unknown_recipe_type_is_error() {
    let err = parse_err(
        r#"
[project]
name = "test"

[recipe.build]
type = "haskell-bin"
script = "cabal build"
"#,
    );
    assert!(!err.is_empty(), "unknown recipe type should fail to parse");
}

#[test]
fn test_missing_project_name_is_error() {
    let err = parse_err(
        r#"
[project]
version = "1.0"
"#,
    );
    assert!(!err.is_empty(), "missing project.name should fail to parse");
}
