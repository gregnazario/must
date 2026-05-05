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

// ── Python recipe types ──────────────────────────────────────────────────────

#[test]
fn test_py_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "py-bin"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::PyBin);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_py_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "py-test"
package = "tests"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::PyTest);
    assert_eq!(cfg.recipe["test"].package.as_deref(), Some("tests"));
}

#[test]
fn test_py_lint_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.lint]
type = "py-lint"
package = "src"
"#,
    );
    assert_eq!(cfg.recipe["lint"].recipe_type, RecipeType::PyLint);
}

// ── Zig recipe types ─────────────────────────────────────────────────────────

#[test]
fn test_zig_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "zig-bin"
package = "install"
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::ZigBin);
    assert_eq!(r.package.as_deref(), Some("install"));
}

#[test]
fn test_zig_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "zig-test"
package = "."
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::ZigTest);
}

// ── Docker recipe types ──────────────────────────────────────────────────────

#[test]
fn test_docker_build_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "docker-build"
image = "myapp:latest"
dockerfile = "Dockerfile"
build_args = ["VERSION=1.0", "PLATFORM=linux"]
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::DockerBuild);
    assert_eq!(r.image.as_deref(), Some("myapp:latest"));
    assert_eq!(r.dockerfile.as_deref(), Some("Dockerfile"));
    assert_eq!(r.build_args, ["VERSION=1.0", "PLATFORM=linux"]);
}

#[test]
fn test_docker_push_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.push]
type = "docker-push"
image = "myregistry/myapp:v2"
deps = ["build"]
"#,
    );
    let r = &cfg.recipe["push"];
    assert_eq!(r.recipe_type, RecipeType::DockerPush);
    assert_eq!(r.image.as_deref(), Some("myregistry/myapp:v2"));
    assert_eq!(r.deps, ["build"]);
}

#[test]
fn test_docker_build_minimal() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "docker-build"
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::DockerBuild);
    assert!(r.image.is_none());
    assert!(r.dockerfile.is_none());
    assert!(r.build_args.is_empty());
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
    assert!(
        cfg.recipe
            .get("hashed")
            .map(|r| r.cache.is_some())
            .unwrap_or(false)
    );
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

// ── Field validation tests ──────────────────────────────────────────────────

#[test]
fn test_validation_shell_missing_script() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("script"),
        "should mention missing 'script': {msg}"
    );
}

#[test]
fn test_validation_rust_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_go_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "go-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_c_bin_missing_sources() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "c-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("sources"),
        "should mention missing 'sources': {msg}"
    );
}

#[test]
fn test_validation_ts_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "ts-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_npm_missing_script() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "npm"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("script"),
        "should mention missing 'script': {msg}"
    );
}

#[test]
fn test_validation_py_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "py-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_zig_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "zig-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_java_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "java-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_kotlin_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "kotlin-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_swift_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "swift-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("package"),
        "should mention missing 'package': {msg}"
    );
}

#[test]
fn test_validation_dotnet_build_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "dotnet-build"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("package"), "should mention missing 'package': {msg}");
}

#[test]
fn test_validation_ruby_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "ruby-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("package"), "should mention missing 'package': {msg}");
}

#[test]
fn test_validation_dart_bin_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "dart-bin"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("package"), "should mention missing 'package': {msg}");
}

#[test]
fn test_validation_elixir_build_missing_package() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "elixir-build"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("package"), "should mention missing 'package': {msg}");
}

#[test]
fn test_validation_docker_build_missing_image() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "docker-build"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("image"),
        "should mention missing 'image': {msg}"
    );
}

#[test]
fn test_validation_docker_push_missing_image() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.push]
type = "docker-push"
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("image"),
        "should mention missing 'image': {msg}"
    );
}

#[test]
fn test_validation_valid_recipe_with_all_fields() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "myapp"

[recipe.test]
type = "rust-test"
package = "myapp"
deps = ["build"]
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_ok());
}

#[test]
fn test_validation_empty_package_rejected() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = ""
"#,
    );
    let result = validate(&cfg, Path::new("Mustfile.toml"));
    assert!(result.is_err(), "empty package should be rejected");
}

// ── Include tests ──────────────────────────────────────────────────────────

#[test]
fn test_include_merges_recipes() {
    use std::io::Write;

    let dir = tempfile::TempDir::new().unwrap();
    let main_path = dir.path().join("Mustfile.toml");
    let inc_path = dir.path().join("libs.toml");

    std::fs::write(
        &inc_path,
        r#"
[recipe.parser]
type = "rust-bin"
package = "parser"

[recipe.parser-test]
type = "rust-test"
package = "parser"
"#,
    )
    .unwrap();

    let mut f = std::fs::File::create(&main_path).unwrap();
    write!(
        f,
        r#"
[project]
name = "myapp"
include = ["libs.toml"]

[recipe.build]
type = "rust-bin"
package = "myapp"
"#,
    )
    .unwrap();

    let cfg = load_config(&main_path).unwrap();
    assert!(cfg.recipe.contains_key("build"), "main recipe present");
    assert!(cfg.recipe.contains_key("parser"), "included recipe present");
    assert!(
        cfg.recipe.contains_key("parser-test"),
        "included recipe present"
    );
    assert_eq!(cfg.project.include, vec!["libs.toml"]);
}

#[test]
fn test_include_main_wins_on_conflict() {
    use std::io::Write;

    let dir = tempfile::TempDir::new().unwrap();
    let main_path = dir.path().join("Mustfile.toml");
    let inc_path = dir.path().join("shared.toml");

    std::fs::write(
        &inc_path,
        r#"
[recipe.build]
type = "shell"
script = "echo included"
"#,
    )
    .unwrap();

    let mut f = std::fs::File::create(&main_path).unwrap();
    write!(
        f,
        r#"
[project]
name = "myapp"
include = ["shared.toml"]

[recipe.build]
type = "rust-bin"
package = "myapp"
"#,
    )
    .unwrap();

    let cfg = load_config(&main_path).unwrap();
    assert_eq!(
        cfg.recipe["build"].recipe_type,
        crate::schema::RecipeType::RustBin,
        "main config should win over include"
    );
}

#[test]
fn test_include_missing_file_is_error() {
    use std::io::Write;

    let dir = tempfile::TempDir::new().unwrap();
    let main_path = dir.path().join("Mustfile.toml");

    let mut f = std::fs::File::create(&main_path).unwrap();
    write!(
        f,
        r#"
[project]
name = "myapp"
include = ["nonexistent.toml"]

[recipe.build]
type = "shell"
script = "echo hi"
"#,
    )
    .unwrap();

    let result = load_config(&main_path);
    assert!(result.is_err(), "missing include should error");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("include"),
        "error should mention include: {msg}"
    );
}

// ── Java recipe types ───────────────────────────────────────────────────────

#[test]
fn test_java_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "java-bin"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::JavaBin);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_java_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "java-test"
package = "services/api"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::JavaTest);
}

// ── Kotlin recipe types ─────────────────────────────────────────────────────

#[test]
fn test_kotlin_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "kotlin-bin"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::KotlinBin);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_kotlin_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "kotlin-test"
package = "libs/core"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::KotlinTest);
}

// ── Swift recipe types ──────────────────────────────────────────────────────

#[test]
fn test_swift_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "swift-bin"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::SwiftBin);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_swift_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "swift-test"
package = "MyPackage"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::SwiftTest);
}

// ── .NET recipe types ───────────────────────────────────────────────────────

#[test]
fn test_dotnet_build_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "dotnet-build"
package = "src/MyApp"
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::DotnetBuild);
    assert_eq!(r.package.as_deref(), Some("src/MyApp"));
}

#[test]
fn test_dotnet_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "dotnet-test"
package = "tests/MyApp.Tests"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::DotnetTest);
}

#[test]
fn test_dotnet_publish_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.publish]
type = "dotnet-publish"
package = "src/MyApp"
"#,
    );
    assert_eq!(cfg.recipe["publish"].recipe_type, RecipeType::DotnetPublish);
}

// ── Ruby recipe types ───────────────────────────────────────────────────────

#[test]
fn test_ruby_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "ruby-bin"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::RubyBin);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_ruby_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "ruby-test"
package = "gems/core"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::RubyTest);
}

// ── Dart recipe types ───────────────────────────────────────────────────────

#[test]
fn test_dart_bin_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "dart-bin"
package = "bin/main.dart"
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::DartBin);
    assert_eq!(r.package.as_deref(), Some("bin/main.dart"));
}

#[test]
fn test_dart_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "dart-test"
package = "test/"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::DartTest);
}

// ── Elixir recipe types ─────────────────────────────────────────────────────

#[test]
fn test_elixir_build_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.build]
type = "elixir-build"
package = "."
"#,
    );
    let r = &cfg.recipe["build"];
    assert_eq!(r.recipe_type, RecipeType::ElixirBuild);
    assert_eq!(r.package.as_deref(), Some("."));
}

#[test]
fn test_elixir_test_recipe() {
    let cfg = parse(
        r#"
[project]
name = "test"

[recipe.test]
type = "elixir-test"
package = "apps/api"
"#,
    );
    assert_eq!(cfg.recipe["test"].recipe_type, RecipeType::ElixirTest);
}
