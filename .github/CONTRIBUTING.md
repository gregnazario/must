# Contributing to Mustfile

Thanks for your interest in contributing! This guide covers everything you need to get started.

## Quick Start

```bash
git clone https://github.com/gregnazario/must.git
cd must
cargo build
cargo test
```

## Development

### Prerequisites

- Rust 1.85+ (edition 2024)
- `cargo-nextest` (optional, for better test output)
- `cargo-llvm-cov` (optional, for coverage reports)

### Building

```bash
cargo build                # debug build
cargo build --release      # release build
```

### Testing

```bash
cargo test                              # run all tests
cargo test -p must-cli                  # CLI tests only
cargo test -p must-recipe-shell         # specific crate
cargo clippy --all-targets              # lint
cargo llvm-cov                          # coverage report
```

### Project Structure

```
must/
├── crates/
│   ├── must-core/          # Core types: BuildContext, Recipe trait, Error, run_command
│   ├── must-cache/         # Disk cache: DiskCache, CacheKey, compute_hash
│   ├── must-config/        # TOML config parsing, validation, 41 RecipeType variants
│   ├── must-graph/         # DAG construction, topo sort, parallel waves
│   ├── must-engine/        # Scheduler, execution engine, progress events
│   ├── must-plugin/        # Lua plugin runtime + stdlib (10 functions)
│   ├── must-import/        # Foreign config import (Makefile, package.json, etc.)
│   ├── must-toolchain/     # Cross-compilation, target triples, container toolchain
│   ├── must-recipe-shell/  # Shell recipe (reference implementation)
│   ├── must-recipe-rust/   # cargo build/test/clippy/doc
│   ├── must-recipe-go/     # go build/test
│   ├── must-recipe-ts/     # tsc, biome lint, npm run
│   ├── must-recipe-py/     # python/pylint/pytest
│   ├── must-recipe-cc/     # cc/c++ build, cmake/make
│   ├── must-recipe-zig/    # zig build/test
│   ├── must-recipe-docker/ # docker build/push
│   ├── must-recipe-java/   # Gradle wrapper build/test
│   ├── must-recipe-kotlin/ # Gradle wrapper build/test
│   ├── must-recipe-swift/  # swift build/test
│   ├── must-recipe-dotnet/ # dotnet build/test/publish
│   ├── must-recipe-ruby/   # bundle install + rspec
│   ├── must-recipe-dart/   # dart compile/test
│   ├── must-recipe-elixir/ # mix deps.get/compile/test
│   ├── must-recipe-flutter/# flutter build/test
│   ├── must-recipe-nim/    # nim compile/test
│   └── must-cli/           # CLI binary, all commands, integration tests
├── examples/               # 19 example Mustfiles
├── action.yml              # GitHub Actions action
└── Cargo.toml              # Workspace root
```

## Adding a New Recipe Type

Follow the existing pattern. Here's the checklist:

### 1. Create the crate

```bash
mkdir -p crates/must-recipe-<lang>/src
```

`crates/must-recipe-<lang>/Cargo.toml`:
```toml
[package]
name = "must-recipe-<lang>"
version = "0.1.0"
edition = "2024"

[dependencies]
must-core = { path = "../must-core" }
must-cache = { path = "../must-cache" }
```

### 2. Implement the `Recipe` trait

```rust
use must_core::{Recipe, BuildContext, RecipeOutput, CacheStrategy, CacheKey, Result, Error};
use must_cache::hash::compute_hash;

pub struct MyBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}
```

Required methods:
- `name()` — recipe name string
- `deps()` — dependency list
- `inputs()` / `outputs()` — file paths for caching
- `cache_strategy()` — `Hash`, `Mtime`, or `Never`
- `cache_key()` — stable key for cache lookups
- `execute()` — the actual build logic

### 3. Required tests

Every recipe crate must include:

| Test | Purpose |
|------|---------|
| `construction` | Verify `new()`, field defaults |
| `deps` | Verify dependency list |
| `cache_strategy` | Check correct strategy variant |
| `cache_key_stable` | Two calls produce same hash |
| `tool_not_found` | Empty env → `ToolNotFound` error |
| `dry_run` | Returns plan without executing |
| `execute_real` | Real execution (guard-skipped if tool missing) |
| `cache_hit` | Store then lookup returns `from_cache: true` |

For real-execution tests, use this pattern:

```rust
fn ctx_with_path() -> BuildContext {
    let mut env = HashMap::new();
    env.insert("PATH".to_string(), std::env::var("PATH").unwrap_or_default());
    env.insert("HOME".to_string(), std::env::var("HOME").unwrap_or_default());
    BuildContext {
        project_root: PathBuf::from("/tmp/test"),
        cache_dir: PathBuf::from("/tmp/test/.cache"),
        log_dir: PathBuf::from("/tmp/test/logs"),
        target: "host".into(),
        profile: "default".into(),
        env,
        dry_run: false,
        parallelism: 1,
    }
}
```

Guard skip when tool is not installed:

```rust
if std::process::Command::new("mytool").arg("--version").output().is_err() {
    return; // skip test
}
```

Handle results resiliently:

```rust
match recipe.execute(&ctx) {
    Ok(out) => assert_eq!(out.recipe_name, "build"),
    Err(must_core::Error::RecipeFailed { .. }) => {} // tool ran but failed
    Err(e) => panic!("unexpected: {e:?}"),
}
```

### 4. Wire into CLI

In `crates/must-cli/src/main.rs`:

1. Add dependency to `Cargo.toml`
2. Add `RecipeType` variant in `crates/must-config/src/schema.rs`
3. Add validation rule in `crates/must-config/src/validate.rs`
4. Add construction + explain + badge + tag logic in `main.rs`

### 5. Add an example

Create `examples/<lang>-app/Mustfile.toml` demonstrating the recipe type.

## Code Style

- Rust 2024 edition (`set_var` requires `unsafe`, string prefix literals need raw strings)
- No comments unless requested
- Follow existing patterns exactly — look at `must-recipe-shell` as the reference
- Clippy must be clean: `cargo clippy --all-targets`
- All tests must pass: `cargo test`

## Commit Messages

Use concise, descriptive messages:

```
add Haskell recipe type
fix cache race in parallel execution
update CI to use Rust 1.85
```

## Pull Request Process

1. Fork and create a feature branch
2. Add tests for your changes
3. Ensure `cargo test` and `cargo clippy --all-targets` pass
4. Open a PR with the PR template filled out
5. One approval required for merge

## Reporting Issues

Use the GitHub issue templates:
- **Bug Report** — unexpected behavior with reproduction steps
- **Feature Request** — new capabilities
- **New Recipe Type** — support for a new language
- **Question** — usage questions

## License

By contributing, you agree that your contributions will be licensed under the same license as the project.
