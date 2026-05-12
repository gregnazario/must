# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-05-05

### Added

#### New recipe types
- `must-recipe-ts`: `ts-bin`, `ts-check`, `ts-lint`, `npm` recipe types for TypeScript/JavaScript projects
- `must-recipe-py`: `py-bin`, `py-test`, `py-lint` recipe types for Python projects (auto-detects uv vs pip)
- `must-recipe-zig`: `zig-bin`, `zig-test` recipe types for Zig projects
- `must-recipe-docker`: `docker-build`, `docker-push` recipe types (auto-detects docker vs podman)

#### New commands
- `must cache list` — list all cached entries with recipe, target, profile, and hash
- `must cache invalidate <recipe>` — clear cache for a specific recipe
- `must cache invalidate --all` — clear all cache entries
- `must cache du` — show cache disk usage
- `must outdated` — show cache status (fresh/stale/missing) for all recipes
- `must init --template` — create Mustfile.toml from 7 templates (minimal, rust, go, python, zig, docker, polyglot)
- `must watch [recipes]` — watch for file changes and rebuild with debounced re-runs
- `must completions <shell>` — generate shell completions for bash, zsh, fish, elvish

#### Features
- **Output capture:** Recipes now capture stdout/stderr via `spawn()` + piped stdio with BufReader threads, streaming live to terminal while also populating `RecipeOutput`
- **Config validation:** Required fields are enforced per recipe type (`shell`→`script`, `rust/go/ts/py/zig`→`package`, `c-bin/c-lib`→`sources`, `docker-*`→`image`)
- **Env interpolation:** `${VAR}` patterns in script, image, dockerfile, ldflags, and build_args are expanded from the composed env map
- **Tool-not-found errors:** All 10 recipe crates catch missing tools and show actionable install hints (e.g. "zig not found: Install Zig: https://ziglang.org/...")
- **Command preview:** `must explain` now shows a `Command:` line with the expanded command for every recipe type
- **Doctor checks:** Added checks for Python 3, pytest, ruff, mypy, Zig, Node.js/npm/npx

#### Examples
- `examples/python-service/` — Python package with py-bin, py-test, py-lint
- `examples/ts-monorepo/` — TypeScript monorepo with ts-bin, ts-check, npm
- `examples/docker-monorepo/` — Multi-service Docker builds with env interpolation
- `examples/zig-tool/` — Zig binary and test recipes

#### Docs
- Updated `USER_GUIDE.md` and `MIGRATION_GUIDE.md` to cover all 19 recipe types

## [0.1.0] - 2026-04-29

### Added

- `must-core`: `Recipe`, `Cache`, `Toolchain` traits; `BuildContext`, `CacheKey`, `CacheLookup`, `RecipeOutput` types; `Error` enum with actionable messages
- `must-config`: TOML schema for `Mustfile.toml`; validation (dep resolution, no cycles)
- `must-graph`: DAG with Kahn's-algorithm topological sort, cycle detection, wave grouping for parallel execution
- `must-engine`: async Tokio scheduler with `-j` parallelism, `--fail-fast`, layered env composition
- `must-cache`: on-disk cache under `.must/cache/`; mtime and SHA-256 hash strategies
- `must-recipe-shell`: generic shell recipe (`sh -c`) with mtime/hash caching
- `must-recipe-rust`: `rust-bin`, `rust-lib`, `rust-test` recipe types via `cargo`
- `must-recipe-go`: `go-bin`, `go-test` recipe types with GOOS/GOARCH cross-compile support
- `must-recipe-cc`: `c-bin`, `c-lib` recipe types (static and shared) via host or cross C compiler
- `must-toolchain`: target triple parsing, Rust/Go/C toolchain discovery, container cross-compilation
- `must-import`: Makefile importer — lexer, parser, translator, TOML writer, report
- `must-cli`: CLI with subcommands `build`, `test`, `list`, `clean`, `explain`, `import`, `doctor`, `graph`
- Global flags: `--dry-run`, `-j`, `--fail-fast`, `--profile`, `--target`
- `must doctor`: environment health check
- `must graph`: dependency graph in text, DOT, and Mermaid formats
- `must import`: converts Makefiles to Mustfile.toml
- `must explain`: cache-key breakdown with inputs, env, hash, and hit/miss status
- GitHub Actions CI (fmt + clippy + test on ubuntu and macos)
- GitHub Actions release workflow (4-platform cross-compile)
