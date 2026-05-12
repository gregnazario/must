# AGENTS.md

Agent instructions for the mustfile project.

## Project Overview

Mustfile is a polyglot build orchestrator written in Rust. One binary, one config (`Mustfile.toml`), consistent verbs across languages. It sits between pure task runners (Make, Just) and full build systems (Bazel, Buck2).

- **41 recipe types** across 17 languages (Rust, Go, C/C++, TypeScript, Python, Zig, Docker, Java, Kotlin, Swift, .NET, Ruby, Dart, Elixir, Flutter, Nim, precompiled binaries, Lua plugins) plus **20 bridge adapters**
- **28 crates** in a Cargo workspace (edition 2024)
- **870+ tests** passing
- **Doc site**: MkDocs Material at `site/` (deployed to mustfile.ai)

## Build & Test Commands

```bash
must build          # cargo build (debug)
must release        # cargo build --release
must test           # cargo test --workspace
must lint           # cargo clippy --workspace --all-targets -- -D warnings
must fmt            # cargo fmt --check
must ci             # fmt + lint + test
must install        # cargo install --path crates/must-cli
```

**IMPORTANT**: After making code changes, always run:
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Architecture

```
crates/
├── must-core/           # Core types, traits (Recipe, Cache), error types, command utils
├── must-config/         # Mustfile.toml parsing, RecipeType enum (41 variants), validation
├── must-graph/          # Dependency DAG, topological sort, wave-based parallel execution
├── must-cache/          # DiskCache (sled-backed), hash/mtime strategies, streaming file hash
├── must-toolchain/      # Toolchain discovery (go, rustc, etc.)
├── must-engine/         # Build engine, scheduler (Arc<BuildContext> + spawn_blocking), env composition
├── must-plugin/         # Lua plugin runtime (mlua 0.11, Mutex<Lua>)
├── must-import/         # Makefile/Justfile/Taskfile → Mustfile.toml converter
├── must-bridge/         # Bridge adapters (20 tools), auto-detect, BridgeRecipe
├── must-cli/            # CLI binary (clap), wires config → recipe instances, 4300+ lines
└── must-recipe-<lang>/  # 15 recipe crates, each implements the Recipe trait
```

### Key Types

- `Recipe` trait (`must-core/src/traits.rs`): `name()`, `recipe_deps()`, `execute(&BuildContext)`, `cache_key()`, `inputs()`, `outputs()`
- `BuildContext` (`must-core/src/types.rs`): `project_root`, `cache_dir`, `log_dir`, `target`, `profile`, `env`, `cache: Option<Arc<dyn Cache>>`
- `RecipeType` enum (`must-config/src/schema.rs`): 41 variants, parsed from `type = "..."` in TOML
- `Recipe` struct (`must-config/src/schema.rs`): config definition with fields like `script`, `scripts`, `package`, `url`, `sha256`, etc.
- `Cache` trait: `lookup()`, `store()`, `invalidate()`
- `CacheStrategy`: `Mtime`, `Hash`, `Never`
- `Engine` (`must-engine/src/scheduler.rs`): resolves deps, runs recipes in parallel waves

## Code Conventions

### Recipe Pattern
Each recipe crate implements `Recipe` trait from `must-core`:
1. Struct with `name`, `deps`, `env` fields
2. Helper function (e.g., `run_cargo`, `run_go`) that creates `Command`, sets `.current_dir(&ctx.project_root)`, calls `run_command()`
3. `execute()` handles dry_run, cache lookup, command execution, cache store
4. `cache_key()` computes hash for cache hits
5. `inputs()`/`outputs()` expand glob patterns

### Testing
- Unit tests in `#[cfg(test)] mod tests` in the same file
- Use `tempfile::TempDir` for filesystem tests
- **Real-execution tests**: use `match` on `Ok`/`ToolNotFound`/`RecipeFailed` to handle missing toolchains gracefully
- **Cache tests**: use `match` pattern (not `.unwrap()`) to avoid flakes
- Use `ctx_with_path()` helper (passes PATH/HOME through `env_clear`)
- Integration tests in `crates/must-cli/tests/integration_tests.rs`

### Code Style
- **NO comments** unless explicitly asked
- Rust 2024 edition: `std::env::set_var` requires `unsafe`, string prefix literals like `.log`/`.lua` need raw strings `r#"..."#`
- Error handling: use `must_core::Error` variants (`ToolNotFound`, `RecipeFailed`, `Config`, `CycleDetected`, etc.)
- Shell commands: `shell_command()` (sh -c on Unix, cmd /C on Windows)
- `run_command()` uses spawn + piped stdio with BufReader threads to stream and capture

### Git Discipline
- **Commit as you go** with logical, atomic commits after each coherent change
- Example cadence: one commit per new crate, one per feature addition, one per bug fix
- Run `cargo clippy` + `cargo test` before each commit to keep the tree green
- Use conventional commit style: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`

## Configuration

### Mustfile.toml
```toml
[project]
name = "my-app"
version = "0.1.0"
include = ["libs/core/Mustfile.toml"]  # optional fragments

[env]
RUST_LOG = "warn"

[env.release]  # profile-scoped env
RUST_LOG = "error"

[recipe.build]
type    = "shell"
script  = "cargo build"
cache   = "hash"  # "hash", "mtime", or "none"
inputs  = ["src/**/*.rs"]
outputs = ["target/debug/myapp"]

[recipe.build.scripts]  # per-OS overrides
macos   = "make -j$(sysctl -n hw.ncpu)"
linux   = "make -j$(nproc)"
"linux.ubuntu" = "make -j$(nproc) DEB=1"  # distro-level from /etc/os-release
win     = "nmake"
```

### Script Resolution Order
1. `scripts.linux.<distro>` (distro from `/etc/os-release`)
2. `scripts.linux` / `scripts.macos` / `scripts.freebsd` (exact OS)
3. `scripts.bsd` or `scripts.unix` (group)
4. `scripts.win` (Windows)
5. `script_win` (Windows shorthand)
6. `script` (default fallback)

## Key Decisions

- `BuildContext.cache: Option<Arc<dyn Cache>>` — DiskCache opened once in CLI, shared by all recipes
- Scheduler wraps `BuildContext` in `Arc`, clones Arc per task (O(1))
- `tokio::task::spawn_blocking` for `recipe.execute()` to avoid blocking tokio workers
- `compose_env_with_base()` accepts pre-computed `std::env::vars()` map
- Streaming file hashing in `compute_hash()` (64KB chunks, O(1) heap)
- Precompiled-bin streams download to `.tmp`, chunked SHA256 verify, then `rename()`
- Lua `Lua` wrapped in `Mutex<Lua>` for Send + Sync
- ShellRecipe sets `.current_dir(&ctx.project_root)` — commands run in project root
- Trivial CLI commands (doctor, completions, init, import) skip tokio runtime creation
- Import auto-detects format from filename (Makefile/justfile/Taskfile.yml)
- `must` falls back to auto-detect bridge mode when no Mustfile.toml exists
- Bridge recipes always use `cache = "none"` — the delegate tool manages caching
- Multi-tool projects: first tool gets standard verbs, others get prefixed (e.g. `npm-build`)
- `compute_hash()` does NOT include `ctx.target` in hash — only struct fields
- `CacheMode::None` serializes as `"none"` (kebab-case), NOT "never"
- All GitHub Actions pinned to commit SHAs

## File Reference

| File | Purpose |
|------|---------|
| `crates/must-core/src/paths.rs` | `ensure_within_root()`, `validate_name_no_traversal()` — path traversal protection |
| `crates/must-core/src/command.rs` | `run_command()`, `shell_command()`, `shell_program()` |
| `crates/must-core/src/types.rs` | `BuildContext`, `CacheStrategy`, `CacheKey`, `RecipeOutput` |
| `crates/must-core/src/traits.rs` | `Recipe` trait, `Cache` trait |
| `crates/must-core/src/error.rs` | `Error` enum (7 variants) |
| `crates/must-config/src/schema.rs` | `RecipeType` (41 variants), `Recipe` struct, `resolved_script()` |
| `crates/must-engine/src/scheduler.rs` | `Engine`, `ExecutionReport`, `ProgressEvent` |
| `crates/must-engine/src/env.rs` | `compose_env()`, `compose_env_with_base()` |
| `crates/must-cache/src/store.rs` | `DiskCache` (sled-backed) |
| `crates/must-cache/src/hash.rs` | `compute_hash()` (streaming, 64KB chunks) |
| `crates/must-graph/src/dag.rs` | `Dag`, `waves()`, `topo_sort()`, `reachable_from()` |
| `crates/must-plugin/src/lib.rs` | `LuaRecipe` (Mutex<Lua>), Lua stdlib |
| `crates/must-bridge/src/detect.rs` | `BridgeTool` (20 adapters), `detect_bridges()`, `auto_config()` |
| `crates/must-bridge/src/bridge.rs` | `BridgeRecipe` — delegates to shell, always `cache = "none"` |
| `crates/must-import/src/lib.rs` | `import()`, `import_justfile()`, `import_taskfile()`, shared `finish_import()` |
| `crates/must-cli/src/main.rs` | CLI entry, all commands, recipe wiring (~4500 lines) |
| `install.sh` | SHA256SUMS-verified install script |
| `site/docs/` | 38+ page MkDocs doc site |

## What's Done

- 41 recipe types, 28 crates, 870+ tests
- 20 bridge adapters: make, npm, gradle, maven, rake, invoke, cmake, cargo-make, ant, just, bazel, buck2, pants, meson, yarn, pnpm, bun, sbt, gulp, nx
- Auto-detect mode: `must` works without Mustfile.toml by detecting build files
- Security: path traversal protection, HTTPS-only precompiled downloads, plugin name validation, log sanitization, secret redaction, pinned GitHub Actions
- Performance: shared Arc cache, spawn_blocking, streaming hash, pre-computed env map, deferred tokio runtime
- Cross-platform: `scripts` table with per-OS and per-distro resolution, `script_win` shorthand
- Import: Makefile, Justfile, Taskfile → Mustfile.toml with auto-format detection
- Docs: 38+ page MkDocs site, rustdoc on all public items, examples for every language

## What's Remaining (Optional)

- New language recipes: Scala, PHP, Haskell, Julia, R, Objective-C
- crates.io publish (needs API token)
- Homebrew tap (needs repo setup)
