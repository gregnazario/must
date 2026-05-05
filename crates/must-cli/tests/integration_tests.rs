use std::time::Duration;

#[test]
fn test_simple_shell_mtime_cache() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Create input file
    std::fs::write(root.join("input.txt"), "hello").unwrap();

    // Write Mustfile.toml
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
inputs = ["input.txt"]
outputs = ["output.txt"]
script = "cp input.txt output.txt"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: output doesn't exist -> should execute
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "first build should succeed");
    assert!(root.join("output.txt").exists(), "output should be created");

    // Get output mtime
    let mtime1 = root
        .join("output.txt")
        .metadata()
        .unwrap()
        .modified()
        .unwrap();

    // Small sleep to ensure mtime would differ if file was rewritten
    std::thread::sleep(Duration::from_millis(50));

    // Second run: input not changed -> should use cache (not rewrite output)
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "second build should succeed");

    let mtime2 = root
        .join("output.txt")
        .metadata()
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        mtime1, mtime2,
        "output should not be rewritten on cache hit"
    );
}

#[test]
fn test_deps_dag_topo_order() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.codegen]
type = "shell"
script = "printf 'codegen\n' >> order.log"

[recipe.build]
type = "shell"
deps = ["codegen"]
script = "printf 'build\n' >> order.log"

[recipe.release]
type = "shell"
deps = ["build"]
script = "printf 'release\n' >> order.log"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "release",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());

    let log = std::fs::read_to_string(root.join("order.log")).unwrap();
    let lines: Vec<&str> = log.trim().lines().collect();
    assert_eq!(lines.len(), 3, "all three recipes should have run: {log}");

    let codegen_pos = lines.iter().position(|&l| l == "codegen").unwrap();
    let build_pos = lines.iter().position(|&l| l == "build").unwrap();
    let release_pos = lines.iter().position(|&l| l == "release").unwrap();

    assert!(codegen_pos < build_pos, "codegen must run before build");
    assert!(build_pos < release_pos, "build must run before release");
}

#[test]
fn test_parallelism() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.a]
type = "shell"
script = "sleep 0.2"

[recipe.b]
type = "shell"
script = "sleep 0.2"

[recipe.c]
type = "shell"
script = "sleep 0.2"

[recipe.d]
type = "shell"
script = "sleep 0.2"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // Time with -j 1 (sequential)
    let t1_start = std::time::Instant::now();
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "-j",
            "1",
            "build",
            "a",
            "b",
            "c",
            "d",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let t1 = t1_start.elapsed();

    // Time with -j 4 (parallel)
    let t4_start = std::time::Instant::now();
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "-j",
            "4",
            "build",
            "a",
            "b",
            "c",
            "d",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let t4 = t4_start.elapsed();

    // -j 4 should be meaningfully faster (2x at minimum)
    assert!(
        t4 < t1 / 2,
        "-j 4 ({:?}) should be much faster than -j 1 ({:?})",
        t4,
        t1
    );
}

#[test]
fn test_rust_bin_recipe() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Create a minimal Cargo workspace with one binary crate
    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[workspace]
resolver = "2"
members = ["hello"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    std::fs::create_dir(root.join("hello")).unwrap();
    std::fs::write(
        root.join("hello").join("Cargo.toml"),
        r#"
[package]
name = "hello"
version.workspace = true
edition.workspace = true

[[bin]]
name = "hello"
path = "src/main.rs"
"#,
    )
    .unwrap();

    std::fs::create_dir(root.join("hello").join("src")).unwrap();
    std::fs::write(
        root.join("hello").join("src").join("main.rs"),
        r#"fn main() { println!("hello from must"); }"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "hello"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: binary doesn't exist → should build
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "first build should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // The built binary should exist
    let built = root.join("target").join("debug").join("hello");
    assert!(
        built.exists(),
        "hello binary should be built at {}",
        built.display()
    );

    // Second run: hash cache should prevent rebuild (0 built, 1 cached)
    let output2 = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output2.status.success(), "second build should succeed");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    assert!(
        stdout2.contains("1 cached") || stdout2.contains("0 built"),
        "second run should use cache: {stdout2}"
    );
}

#[test]
fn test_shell_hash_cache_content_sensitive() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(root.join("input.txt"), "version 1").unwrap();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.process]
type = "shell"
inputs = ["input.txt"]
outputs = ["output.txt"]
cache = "hash"
script = "cp input.txt output.txt"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: cache miss → execute
    let s1 = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "process",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(s1.success());
    assert!(root.join("output.txt").exists());

    // Second run: same content → cache hit (output unchanged)
    let out_mtime1 = root
        .join("output.txt")
        .metadata()
        .unwrap()
        .modified()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let s2 = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "process",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(s2.success());
    let out_mtime2 = root
        .join("output.txt")
        .metadata()
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        out_mtime1, out_mtime2,
        "hash cache: unchanged content should not rerun"
    );

    // Change file content → should rebuild
    std::fs::write(root.join("input.txt"), "version 2").unwrap();
    let s3 = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "process",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(s3.success());
    let output_content = std::fs::read_to_string(root.join("output.txt")).unwrap();
    assert_eq!(
        output_content.trim(),
        "version 2",
        "output should reflect new content after rebuild"
    );
}

#[test]
fn test_go_bin_recipe() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Skip if Go is not installed
    let go_available = std::process::Command::new("go")
        .arg("version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !go_available {
        eprintln!("skipping test_go_bin_recipe: go not installed");
        return;
    }

    // Write a minimal main.go
    std::fs::write(
        root.join("main.go"),
        r#"package main
import "fmt"
func main() { fmt.Println("hello from go") }
"#,
    )
    .unwrap();

    // Write go.mod
    std::fs::write(
        root.join("go.mod"),
        r#"module example.com/hello
go 1.21
"#,
    )
    .unwrap();

    // Write Mustfile.toml
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "gotest"

[recipe.build]
type = "go-bin"
package = "."
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "go-bin build should succeed");
}

#[test]
fn test_multi_target_expansion() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Write Mustfile.toml with a [targets] section and a shell recipe
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "multitarget"

[targets]
release = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]

[recipe.build]
type = "shell"
script = "echo building for target"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "--target",
            "release",
        ])
        .current_dir(root)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "multi-target build should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("[target: x86_64-unknown-linux-gnu]"),
        "expected x86_64 target header in output\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("[target: aarch64-unknown-linux-gnu]"),
        "expected aarch64 target header in output\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn test_c_bin_recipe_local() {
    // Skip if no C compiler available on host
    let cc_available = std::process::Command::new("cc")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || std::process::Command::new("gcc")
            .arg("--version")
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
    if !cc_available {
        eprintln!("skipping test_c_bin_recipe_local: no C compiler found");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    // Write a minimal C program
    std::fs::write(
        root.join("hello.c"),
        r#"
#include <stdio.h>
int main(void) { puts("hello from c"); return 0; }
"#,
    )
    .unwrap();

    // Write Mustfile.toml
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "ctest"

[recipe.build]
type = "c-bin"
sources = ["hello.c"]
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let status = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy()])
        .arg("build")
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "c-bin build should succeed");
    assert!(
        root.join("build").join("build").exists() || root.join("build").exists(),
        "build output directory should exist"
    );
}

#[test]
fn test_c_bin_recipe_container() {
    // Skip if docker/podman not available
    let docker_available = std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::process::Command::new("podman")
            .arg("info")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !docker_available {
        eprintln!("skipping test_c_bin_recipe_container: no container runtime found");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("hello.c"),
        r#"
#include <stdio.h>
int main(void) { puts("hello from container"); return 0; }
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "ctest"

[recipe.build]
type = "c-bin"
sources = ["hello.c"]

[recipe.build.cross]
"x86_64-unknown-linux-gnu" = { cross = "container" }
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let status = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy()])
        .args(["--target", "x86_64-unknown-linux-gnu"])
        .arg("build")
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "c-bin container build should succeed");
}

#[test]
fn test_list_command() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "listtest"

[recipe.build]
type = "shell"
script = "echo build"

[recipe.test]
type = "shell"
deps = ["build"]
script = "echo test"

[recipe.lint]
type = "shell"
script = "echo lint"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "list",
        ])
        .current_dir(root)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "must list should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("build"),
        "stdout should contain 'build': {stdout}"
    );
    assert!(
        stdout.contains("test"),
        "stdout should contain 'test': {stdout}"
    );
    assert!(
        stdout.contains("lint"),
        "stdout should contain 'lint': {stdout}"
    );
    assert!(
        stdout.contains("shell"),
        "stdout should contain 'shell' (type column): {stdout}"
    );
}

#[test]
fn test_explain_command() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "explaintest"

[recipe.build]
type = "shell"
script = "echo build"
cache = "hash"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "explain",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "must explain should exit 0\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("Cache key"),
        "stdout should contain 'Cache key': {stdout}"
    );
    assert!(
        stdout.contains("build"),
        "stdout should contain 'build' (recipe name): {stdout}"
    );
    assert!(
        stdout.contains("Strategy") || stdout.contains("hash"),
        "stdout should contain 'Strategy' or 'hash': {stdout}"
    );
}

#[test]
fn test_dry_run_flag() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "dryruntest"

[recipe.build]
type = "shell"
script = "touch sentinel_file.txt"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "--dry-run",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "must build --dry-run should exit 0\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        !root.join("sentinel_file.txt").exists(),
        "sentinel_file.txt should NOT exist after --dry-run"
    );
}

#[test]
fn test_clean_cache_command() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"[project]
name = "cleantest"

[recipe.build]
type = "shell"
cache = "hash"
script = "echo cached"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: populate the cache
    let status = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "initial build should succeed");

    // Cache directory should now exist
    let cache_dir = root.join(".mustfile").join("cache");
    assert!(
        cache_dir.exists(),
        "cache directory should exist after first build: {}",
        cache_dir.display()
    );

    // Run must clean --cache
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "clean",
            "--cache",
        ])
        .current_dir(root)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "must clean --cache should exit 0\nstdout: {stdout}\nstderr: {stderr}"
    );

    // Cache directory should no longer exist
    assert!(
        !cache_dir.exists(),
        "cache directory should be removed after must clean --cache: {}",
        cache_dir.display()
    );
}

#[test]
fn test_test_subcommand() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.test]
type = "shell"
script = "touch test_ran.txt"
"#,
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "test",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(root.join("test_ran.txt").exists());
}

#[test]
fn test_unknown_recipe_exits_nonzero() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo ok"
"#,
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
            "nonexistent",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(!status.success(), "should exit nonzero for unknown recipe");
}

#[test]
fn test_resolve_targets_group_expansion() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[targets]
mygroup = ["host"]

[recipe.build]
type = "shell"
script = "echo building"
"#,
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "--target",
            "mygroup",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "group target should expand correctly"
    );
}

#[test]
fn test_explain_recipe_with_inputs() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("hello.txt"), "hello").unwrap();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
inputs = ["hello.txt"]
outputs = ["out.txt"]
script = "cp hello.txt out.txt"
"#,
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "explain",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Inputs:"), "should show inputs section");
    assert!(stdout.contains("hello.txt"), "should list the input file");
}

#[test]
fn test_explain_unknown_recipe_exits_nonzero() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo ok"
"#,
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "explain",
            "bogus",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(!status.success(), "should exit nonzero for unknown recipe");
}

#[test]
fn test_failing_recipe_exits_nonzero() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "exit 1"
"#,
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "failing recipe should cause nonzero exit"
    );
}

#[test]
fn test_clean_when_no_cache_dir() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo ok"
"#,
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "clean",
            "--cache",
        ])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "clean --cache on nonexistent cache should succeed"
    );
}

#[test]
fn test_list_shows_deps() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
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
"#,
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "list",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("codegen"), "should list codegen recipe");
    assert!(
        stdout.contains("build"),
        "should list build recipe with dep"
    );
}

#[test]
fn test_explain_recipe_with_env_vars() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[env.default]
MY_BUILD_VAR = "some_value"

[recipe.build]
type = "shell"
script = "echo ok"
"#,
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "explain",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The env var MY_BUILD_VAR should appear in the "Env (affects hash):" section
    assert!(
        stdout.contains("MY_BUILD_VAR"),
        "should show env var: {stdout}"
    );
}

#[test]
fn test_build_with_two_targets_shows_header() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo building"
"#,
    )
    .unwrap();
    // Pass two distinct target strings so we get target headers in output.
    // x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu are both shell-compatible
    // (shell recipe doesn't care about target).
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_must"))
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "--target",
            "x86_64-unknown-linux-gnu",
            "--target",
            "aarch64-unknown-linux-gnu",
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should contain "[target: ...]" headers since 2 targets
    assert!(
        stdout.contains("[target:"),
        "expected target headers, got: {stdout}"
    );
}

#[test]
fn test_py_bin_recipe() {
    let python3_available = std::process::Command::new("python3")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !python3_available {
        eprintln!("skipping test_py_bin_recipe: python3 not installed");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("pyproject.toml"),
        r#"[project]
name = "hello-py"
version = "0.1.0"
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("hello_py")).unwrap();
    std::fs::write(root.join("hello_py/__init__.py"), "").unwrap();

    // Create a virtual environment so uv/pip doesn't complain about externally-managed Python
    let venv_status = std::process::Command::new("python3")
        .args(["-m", "venv", ".venv"])
        .current_dir(root)
        .status()
        .unwrap();
    if !venv_status.success() {
        eprintln!("skipping test_py_bin_recipe: could not create venv");
        return;
    }

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "pytest"

[recipe.build]
type = "py-bin"
package = "."

[recipe.build.env]
VIRTUAL_ENV = ".venv"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "py-bin build should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn test_py_test_recipe() {
    let pytest_available = std::process::Command::new("pytest")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !pytest_available {
        eprintln!("skipping test_py_test_recipe: pytest not installed");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("test_basic.py"),
        r#"def test_ok():
    assert True
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "pytest"

[recipe.test]
type = "py-test"
package = "."
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "test",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "py-test should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn test_zig_bin_recipe() {
    let zig_available = std::process::Command::new("zig")
        .arg("version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !zig_available {
        eprintln!("skipping test_zig_bin_recipe: zig not installed");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("build.zig"),
        r#"const std = @import("std");
pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const exe = b.addExecutable(.{
        .name = "hello",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
        }),
    });
    b.installArtifact(exe);
}
"#,
    )
    .unwrap();

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src").join("main.zig"),
        r#"const std = @import("std");
pub fn main() !void {
    std.debug.print("hello from zig\n", .{});
}
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "zigtest"

[recipe.build]
type = "zig-bin"
package = "install"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "zig-bin build should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn test_docker_build_recipe() {
    let docker_available = std::process::Command::new("docker")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        || std::process::Command::new("podman")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !docker_available {
        eprintln!("skipping test_docker_build_recipe: no container runtime found");
        return;
    }

    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();

    std::fs::write(
        root.join("Dockerfile"),
        r#"FROM alpine:latest
RUN echo "hello from docker"
"#,
    )
    .unwrap();

    std::fs::write(
        root.join("Mustfile.toml"),
        r#"
[project]
name = "dockertest"

[recipe.build]
type = "docker-build"
image = "mustfile-test-docker-build:latest"
dockerfile = "Dockerfile"
"#,
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_must");
    let output = std::process::Command::new(binary)
        .args([
            "--file",
            &root.join("Mustfile.toml").to_string_lossy(),
            "build",
        ])
        .current_dir(root)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "docker-build should succeed\nstdout: {stdout}\nstderr: {stderr}"
    );
}
