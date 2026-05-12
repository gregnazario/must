# M6 Plan — Polish / v1

## Goal

Bring mustfile to a shippable v1 state: two new utility subcommands (`doctor`, `graph`), GitHub Actions CI + release workflows, a user guide, and a changelog.

## Worktree

`/Users/greg/git/mustfile/.worktrees/polish/` (branch `polish`)

## Codebase context

- Workspace root: `crates/` — 12 crates
- CLI entry: `crates/must-cli/src/main.rs` — `Commands` enum, `run()` function
- Toolchain probes: `must-toolchain::discover` — `rust_target_installed`, `go_installed`, `c_compiler_available`, `detect_runtime` (container)
- Graph: `must-graph::Dag` — has `waves()` and `topo_sort()` methods; `deps` field via `HashMap<String, Vec<String>>`
- Config: `must-config::load_config(&path)` returns `Config` with `Config.recipe` map and `Config.targets`
- Error types: in `must-core/src/error.rs` — already have actionable messages (Config, CycleDetected, UnknownRecipe, RecipeFailed, ToolchainNotFound, Cache, Io)
- No `.github/` directory yet

## Tasks

---

### Task 1: `must doctor` subcommand

Add a `Doctor` subcommand to `must-cli`. It checks the local environment and prints a health report with ✓/✗ and actionable hints.

**Files to modify:**
- `crates/must-cli/Cargo.toml`: add `must-toolchain = { path = "../must-toolchain" }` (if not already present — check first)
- `crates/must-cli/src/main.rs`

**Add to `Commands` enum:**
```rust
/// Check environment health (toolchains, container runtime, cache)
Doctor,
```

**Handler in `run()` — early return before config load (like Import):**

```rust
if matches!(cli.command, Commands::Doctor) {
    run_doctor();
    return Ok(());
}
```

**`run_doctor()` function (standalone, not async):**

```rust
fn run_doctor() {
    use must_toolchain::discover::{go_installed, go_install_hint, rust_target_installed, rust_install_hint, c_compiler_available, c_install_hint};
    use must_toolchain::container::detect_runtime;
    use must_toolchain::triple::Triple;

    let mut ok = true;

    println!("must doctor — environment health check\n");

    // 1. Rust / cargo
    let rust_ok = std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    print_check("Rust/cargo", rust_ok, "Install from https://rustup.rs");
    ok &= rust_ok;

    // 2. Go
    let go_ok = go_installed();
    print_check("Go", go_ok, &go_install_hint());
    // Go is optional — don't fail overall for missing Go

    // 3. C compiler (host)
    let host = Triple { raw: "host".into(), os: must_toolchain::triple::Os::Linux, arch: must_toolchain::triple::Arch::X86_64 };
    let cc_ok = c_compiler_available(&host);
    print_check("C compiler (host)", cc_ok, &c_install_hint(&host));
    // C is optional

    // 4. Container runtime
    let container = detect_runtime();
    let container_ok = container.is_some();
    let container_name = container.as_ref().map(|r| format!("{:?}", r)).unwrap_or_default();
    let container_msg = if container_ok {
        format!("found: {container_name}")
    } else {
        "not found".into()
    };
    println!("  {} Container runtime  — {}", if container_ok { "✓" } else { "?" }, container_msg);
    if !container_ok {
        println!("    hint: Install Docker (https://docker.com) or Podman (https://podman.io) for container cross-compilation");
    }

    // 5. Cache health
    let cache_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".must")
        .join("cache");
    if cache_dir.exists() {
        let size = dir_size(&cache_dir).unwrap_or(0);
        println!("  ✓ Cache             — {:.1} MB at {}", size as f64 / 1_048_576.0, cache_dir.display());
    } else {
        println!("  ✓ Cache             — empty (no cache yet)");
    }

    println!();
    if ok {
        println!("All required tools present. Ready to build.");
    } else {
        println!("Some required tools are missing. See hints above.");
        std::process::exit(1);
    }
}

fn print_check(label: &str, ok: bool, hint: &str) {
    let icon = if ok { "✓" } else { "✗" };
    println!("  {icon} {label:<20}");
    if !ok {
        println!("    hint: {hint}");
    }
}

fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            total += dir_size(&entry.path()).unwrap_or(0);
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}
```

**Note on Triple construction:** The `Triple` struct has fields `raw`, `os`, `arch`. For the host check, construct it with:
```rust
Triple { raw: "host".to_string(), os: Os::Linux, arch: Arch::X86_64 }
```
But first read `must-toolchain/src/triple.rs` to confirm the struct fields and enum variants. Adjust accordingly.

**Alternative if Triple construction is complex:** Just call `c_compiler_available` with the host triple or use a simpler approach — check if `cc` or `gcc` is in PATH directly:
```rust
let cc_ok = std::process::Command::new("cc").arg("--version").output()
    .map(|o| o.status.success()).unwrap_or(false)
    || std::process::Command::new("gcc").arg("--version").output()
    .map(|o| o.status.success()).unwrap_or(false);
```

**Tests (in `must-cli/src/main.rs` test block):**
```rust
#[test]
fn test_dir_size_returns_zero_for_empty_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert_eq!(dir_size(tmp.path()).unwrap(), 0);
}

#[test]
fn test_dir_size_counts_file_bytes() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.txt"), "hello").unwrap();
    assert_eq!(dir_size(tmp.path()).unwrap(), 5);
}

#[test]
fn test_print_check_does_not_panic() {
    print_check("test", true, "no hint needed");
    print_check("test", false, "install something");
}
```

**Verification:** `cargo test --workspace` passes; `cargo run -p must-cli -- doctor --help` works.

**Commit:** `feat(must-cli): add doctor subcommand`

---

### Task 2: `must graph` subcommand

Add a `Graph` subcommand that loads `Mustfile.toml` and prints the dependency graph.

**Files to modify:**
- `crates/must-cli/src/main.rs`

**Add to `Commands` enum:**
```rust
/// Print the recipe dependency graph
Graph {
    /// Output format: text, dot, or mermaid
    #[arg(long, default_value = "text")]
    format: String,
},
```

**Handler — add to the `match cli.command` block (requires config, so NOT an early return):**

```rust
Commands::Graph { format } => {
    print_graph(&config, &format)?;
}
```

**`print_graph()` function:**

```rust
fn print_graph(config: &Config, format: &str) -> must_core::Result<()> {
    let dep_map: HashMap<String, Vec<String>> = config
        .recipe
        .iter()
        .map(|(name, r)| (name.clone(), r.deps.clone()))
        .collect();
    let dag = Dag::new(dep_map.clone());
    let order = dag.topo_sort()?;

    match format {
        "dot" => {
            println!("digraph mustfile {{");
            println!("  rankdir=LR;");
            for name in &order {
                if let Some(deps) = dep_map.get(name) {
                    for dep in deps {
                        println!("  \"{name}\" -> \"{dep}\";");
                    }
                }
            }
            println!("}}");
        }
        "mermaid" => {
            println!("graph LR");
            for name in &order {
                if let Some(deps) = dep_map.get(name) {
                    for dep in deps {
                        println!("  {name} --> {dep}");
                    }
                }
            }
        }
        _ => {
            // text (default)
            println!("Recipe dependency graph:\n");
            for name in &order {
                let deps = dep_map.get(name).map(|d| d.as_slice()).unwrap_or(&[]);
                if deps.is_empty() {
                    println!("  {name}");
                } else {
                    println!("  {name} <- [{}]", deps.join(", "));
                }
            }
        }
    }
    Ok(())
}
```

**Tests:**
```rust
#[test]
fn test_print_graph_text_format() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mustfile = tmp.path().join("Mustfile.toml");
    std::fs::write(&mustfile, r#"
[project]
name = "test"

[recipe.build]
type = "shell"
script = "echo build"

[recipe.test]
type = "shell"
deps = ["build"]
script = "echo test"
"#).unwrap();
    let config = must_config::load_config(&mustfile).unwrap();
    assert!(print_graph(&config, "text").is_ok());
}

#[test]
fn test_print_graph_dot_format() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mustfile = tmp.path().join("Mustfile.toml");
    std::fs::write(&mustfile, "[project]\nname=\"test\"\n[recipe.build]\ntype=\"shell\"\nscript=\"echo ok\"\n").unwrap();
    let config = must_config::load_config(&mustfile).unwrap();
    assert!(print_graph(&config, "dot").is_ok());
}

#[test]
fn test_print_graph_mermaid_format() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mustfile = tmp.path().join("Mustfile.toml");
    std::fs::write(&mustfile, "[project]\nname=\"test\"\n[recipe.build]\ntype=\"shell\"\nscript=\"echo ok\"\n").unwrap();
    let config = must_config::load_config(&mustfile).unwrap();
    assert!(print_graph(&config, "mermaid").is_ok());
}

#[test]
fn test_print_graph_cycle_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mustfile = tmp.path().join("Mustfile.toml");
    std::fs::write(&mustfile, r#"
[project]
name = "test"

[recipe.a]
type = "shell"
deps = ["b"]
script = "echo a"

[recipe.b]
type = "shell"
deps = ["a"]
script = "echo b"
"#).unwrap();
    let config = must_config::load_config(&mustfile).unwrap();
    assert!(print_graph(&config, "text").is_err());
}
```

**Verification:** `cargo test --workspace` passes; `cargo run -p must-cli -- graph --help` works.

**Commit:** `feat(must-cli): add graph subcommand`

---

### Task 3: GitHub Actions CI + release workflows

Create two workflow files.

**Files to create:**
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`

Note: The `.github/` directory does not exist yet. Create it with `mkdir -p`.

**`.github/workflows/ci.yml`:**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: Test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: cargo fmt check
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: cargo test
        run: cargo test --workspace
```

**`.github/workflows/release.yml`:**

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: must-x86_64-linux
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: must-aarch64-linux
            cross: true
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: must-x86_64-macos
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: must-aarch64-macos
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - name: Install cross (for cross-compilation)
        if: matrix.cross
        run: cargo install cross --git https://github.com/cross-rs/cross
      - name: Build (native)
        if: "!matrix.cross"
        run: cargo build --release --target ${{ matrix.target }} -p must-cli
      - name: Build (cross)
        if: matrix.cross
        run: cross build --release --target ${{ matrix.target }} -p must-cli
      - name: Prepare artifact
        run: |
          cp target/${{ matrix.target }}/release/must ${{ matrix.artifact }}
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: ${{ matrix.artifact }}

  release:
    name: Create Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with:
          path: artifacts
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*
          generate_release_notes: true
```

**Verification:** Files exist and are valid YAML (no cargo verification needed — these are CI config files). Run `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` or similar to validate syntax if Python is available. Otherwise just ensure they parse as valid YAML mentally.

**Commit:** `ci: add GitHub Actions CI and release workflows`

---

### Task 4: CHANGELOG.md and USER_GUIDE.md

Create two documentation files.

**`CHANGELOG.md`** — at repo root, following Keep a Changelog format:

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-29

### Added
- `must-core`: `Recipe`, `Cache`, `Toolchain` traits; `BuildContext`, `CacheKey`, `CacheLookup`, `RecipeOutput` types; `Error` enum
- `must-config`: TOML schema for `Mustfile.toml`; validation (name uniqueness, dep resolution)
- `must-graph`: DAG with Kahn's-algorithm topological sort, cycle detection, wave grouping for parallel execution
- `must-engine`: async Tokio scheduler with `-j` parallelism, `--fail-fast`, layered env composition
- `must-cache`: on-disk cache under `.must/cache/`; mtime and SHA-256 hash strategies
- `must-recipe-shell`: generic shell recipe (`sh -c`) with mtime/hash caching
- `must-recipe-rust`: `rust-bin`, `rust-lib`, `rust-test` recipe types via `cargo`
- `must-recipe-go`: `go-bin`, `go-test` recipe types with GOOS/GOARCH cross-compile support
- `must-recipe-cc`: `c-bin`, `c-lib` recipe types (static and shared) via host or cross C compiler
- `must-toolchain`: target triple parsing, Rust/Go/C toolchain discovery, container cross-compilation (Docker/Podman)
- `must-import`: Makefile importer — lexer, AST parser, translator, TOML writer, Markdown report
- `must-cli`: CLI entry point with subcommands `build`, `test`, `list`, `clean`, `explain`, `import`, `doctor`, `graph`
- `--dry-run`, `-j`, `--fail-fast`, `--profile`, `--target` global flags
- `must doctor`: environment health check for Rust, Go, C compiler, container runtime, cache
- `must graph`: dependency graph in text, DOT, and Mermaid formats
- `must import`: converts Makefiles to `Mustfile.toml` with a diff report
- `must explain`: cache-key breakdown showing inputs, env vars, hash, and hit/miss status
- GitHub Actions CI (fmt, clippy, test on ubuntu + macos) and release workflows (4-platform matrix builds)
```

**`docs/USER_GUIDE.md`** — comprehensive user guide:

```markdown
# must — User Guide

`must` is a polyglot build orchestrator. It reads a `Mustfile.toml`, resolves recipe dependencies, and executes recipes in parallel using mtime or hash-based caching.

## Installation

```bash
cargo install --locked mustfile
```

Or download a prebuilt binary from the [releases page](https://github.com/greg/mustfile/releases).

## Quick start

Create a `Mustfile.toml` in your project root:

```toml
[project]
name = "my-project"

[recipe.build]
type = "shell"
script = "gcc -o app main.c"
inputs = ["main.c"]
outputs = ["app"]

[recipe.test]
type = "shell"
deps = ["build"]
script = "./app --test"
```

Then run:

```bash
must build    # builds 'build' recipe
must test     # builds 'test' and its deps
must list     # shows all recipes
```

## Mustfile.toml reference

### `[project]`

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Project name |
| `version` | string (optional) | Version string |

### `[env.global]`

Key-value pairs set in every recipe's environment:

```toml
[env.global]
CC = "gcc"
CFLAGS = "-Wall -O2"
```

### `[recipe.<name>]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `type` | string | required | Recipe type: `shell`, `rust-bin`, `rust-lib`, `rust-test`, `go-bin`, `go-test`, `c-bin`, `c-lib` |
| `deps` | list | `[]` | Recipe names that must complete first |
| `inputs` | list | `[]` | Glob patterns for input files (mtime tracking) |
| `outputs` | list | `[]` | Glob patterns for output files (mtime tracking) |
| `script` | string | — | Shell script (type = `shell` only) |
| `cache` | string | type-default | `"mtime"`, `"hash"`, or `"none"` |
| `phony` | bool | `false` | Always re-run even if outputs are up to date |
| `env` | table | `{}` | Extra env vars for this recipe only |
| `package` | string | — | Package name (Rust/Go recipes) |
| `features` | list | `[]` | Cargo features (rust-* recipes) |
| `ldflags` | string | — | Linker flags (`go-bin` only) |
| `sources` | list | `[]` | Source files (`c-bin`, `c-lib`) |
| `includes` | list | `[]` | Include directories (`c-bin`, `c-lib`) |
| `link_libs` | list | `[]` | Libraries to link (`c-bin`, `c-lib`) |

### `[targets]`

Named groups of cross-compile targets:

```toml
[targets]
linux = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
```

## CLI reference

### `must build [recipes...]`

Build the named recipes (default: `build`) and all their dependencies.

```bash
must build                    # run 'build' recipe
must build codegen proto      # run 'codegen' and 'proto' recipes
must build --target linux     # cross-compile for all 'linux' targets
must build -j 8               # use 8 parallel workers
must build --dry-run          # show what would run, without running
```

### `must test [recipes...]`

Same as `build` but defaults to the `test` recipe.

### `must list`

Print all recipes with their type and dependencies.

### `must explain <recipe>`

Show why a recipe will or won't rebuild: cache strategy, input files with hashes, env vars, computed cache key, and hit/miss status.

```bash
must explain build
```

### `must import`

Convert a Makefile to a `Mustfile.toml` starter file.

```bash
must import                          # reads ./Makefile, writes ./Mustfile.toml
must import --makefile build/GNUmakefile --out build/Mustfile.toml
```

Produces a `MUSTFILE_IMPORT_REPORT.md` alongside the output listing what was translated, what needs manual attention (pattern rules, includes), and what was skipped.

### `must doctor`

Check whether all required tools are installed and the cache is healthy.

```bash
must doctor
```

### `must graph`

Print the recipe dependency graph.

```bash
must graph                    # text format
must graph --format dot       # Graphviz DOT — pipe to dot -Tpng
must graph --format mermaid   # Mermaid diagram
```

### `must clean`

Remove recipe output files.

```bash
must clean          # clean outputs
must clean --cache  # also wipe the .must/cache directory
```

## Caching

By default:
- Shell recipes use **mtime** caching: rebuild if any input file is newer than any output file.
- First-class recipes (Rust, Go, C) use **hash** caching: rebuild if the SHA-256 hash of inputs + env + flags changes.

Override per recipe:

```toml
[recipe.codegen]
type = "shell"
cache = "hash"   # use hash caching instead
script = "python gen.py"
inputs = ["schema.proto"]
outputs = ["gen/"]
```

## Cross-compilation

Add `--target <triple>` to any build command:

```bash
must build --target aarch64-unknown-linux-gnu
```

Or use a named group from `[targets]`:

```bash
must build --target linux   # builds all targets in the 'linux' group
```

For C cross-compilation, `must` looks for `<triple>-gcc` in PATH. For Rust, it uses `cargo` with `CARGO_TARGET_<TRIPLE>_LINKER`. For Go, it sets `GOOS`/`GOARCH` from the triple.

## Profiles

Add per-profile env overrides:

```toml
[env.release]
CARGO_PROFILE = "release"
```

Then: `must build --profile release`
```

**Note on CHANGELOG.md date:** Use `2026-04-29` as the release date (today's date from CLAUDE.md context).

**Verification:** Files exist and are well-formed Markdown. No cargo verification needed.

**Commit:** `docs: add CHANGELOG.md and USER_GUIDE.md`

---

### Task 5: Update TASKS.md and tag v1.0.0

Mark M6 tasks as done in `docs/TASKS.md`, then create the v1.0.0 tag.

**Files to modify:**
- `docs/TASKS.md`: mark all M6 items as `[x]` or `[-]` (deferred items stay `[-]`)

**M6 completion status:**
- `[x]` `must doctor` ✓
- `[x]` `must graph` ✓
- `[x]` Error message audit (already actionable in must-core/src/error.rs)
- `[-]` `--explain` polish (deferred — current implementation is sufficient for v1)
- `[x]` Cross-platform paths (sh -c on unix already works on macOS + Linux)
- `[x]` GitHub Actions CI
- `[x]` GitHub Actions release workflow
- `[-]` `cargo install --locked mustfile` (requires crates.io publish — deferred)
- `[-]` Prebuilt binaries (workflow created; actual publish deferred until pushed)
- `[-]` `install.sh` (deferred)
- `[x]` `docs/USER_GUIDE.md`
- `[x]` CHANGELOG.md
- `[-]` Tag v1.0.0 (done at end of this task)

**After updating TASKS.md, create the tag:**
```bash
git tag -a v1.0.0 -m "v1.0.0 — polyglot build orchestrator with shell, Rust, Go, C recipes, Makefile import, doctor, and graph"
```

**Commit:** `chore: mark M6 complete in TASKS.md`
(Tag after commit.)

**Verification:** `git tag` shows `v1.0.0`; `cargo test --workspace` still passes.

---

## Commit order

1. Task 1: `feat(must-cli): add doctor subcommand`
2. Task 2: `feat(must-cli): add graph subcommand`
3. Task 3: `ci: add GitHub Actions CI and release workflows`
4. Task 4: `docs: add CHANGELOG.md and USER_GUIDE.md`
5. Task 5: `chore: mark M6 complete in TASKS.md` + tag `v1.0.0`
