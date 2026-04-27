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
    std::fs::write(root.join("Cargo.toml"), r#"
[workspace]
resolver = "2"
members = ["hello"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#).unwrap();

    std::fs::create_dir(root.join("hello")).unwrap();
    std::fs::write(root.join("hello").join("Cargo.toml"), r#"
[package]
name = "hello"
version.workspace = true
edition.workspace = true

[[bin]]
name = "hello"
path = "src/main.rs"
"#).unwrap();

    std::fs::create_dir(root.join("hello").join("src")).unwrap();
    std::fs::write(root.join("hello").join("src").join("main.rs"),
        r#"fn main() { println!("hello from must"); }"#
    ).unwrap();

    std::fs::write(root.join("Mustfile.toml"), r#"
[project]
name = "test"

[recipe.build]
type = "rust-bin"
package = "hello"
"#).unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: binary doesn't exist → should build
    let output = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "first build should succeed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));

    // The built binary should exist
    let built = root.join("target").join("debug").join("hello");
    assert!(built.exists(), "hello binary should be built at {}", built.display());

    // Second run: hash cache should prevent rebuild (0 built, 1 cached)
    let output2 = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build"])
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
    std::fs::write(root.join("Mustfile.toml"), r#"
[project]
name = "test"

[recipe.process]
type = "shell"
inputs = ["input.txt"]
outputs = ["output.txt"]
cache = "hash"
script = "cp input.txt output.txt"
"#).unwrap();

    let binary = env!("CARGO_BIN_EXE_must");

    // First run: cache miss → execute
    let s1 = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build", "process"])
        .current_dir(root)
        .status().unwrap();
    assert!(s1.success());
    assert!(root.join("output.txt").exists());

    // Second run: same content → cache hit (output unchanged)
    let out_mtime1 = root.join("output.txt").metadata().unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));
    let s2 = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build", "process"])
        .current_dir(root)
        .status().unwrap();
    assert!(s2.success());
    let out_mtime2 = root.join("output.txt").metadata().unwrap().modified().unwrap();
    assert_eq!(out_mtime1, out_mtime2, "hash cache: unchanged content should not rerun");

    // Change file content → should rebuild
    std::fs::write(root.join("input.txt"), "version 2").unwrap();
    let s3 = std::process::Command::new(binary)
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build", "process"])
        .current_dir(root)
        .status().unwrap();
    assert!(s3.success());
    let output_content = std::fs::read_to_string(root.join("output.txt")).unwrap();
    assert_eq!(output_content.trim(), "version 2", "output should reflect new content after rebuild");
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
        .args(["--file", &root.join("Mustfile.toml").to_string_lossy(), "build"])
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
