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
