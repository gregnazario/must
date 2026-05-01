# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-29

### Added
- `must-core`: `Recipe`, `Cache`, `Toolchain` traits; `BuildContext`, `CacheKey`, `CacheLookup`, `RecipeOutput` types; `Error` enum with actionable messages
- `must-config`: TOML schema for `Mustfile.toml`; validation (name uniqueness, dep resolution, no cycles)
- `must-graph`: DAG with Kahn's-algorithm topological sort, cycle detection, wave grouping for parallel execution
- `must-engine`: async Tokio scheduler with `-j` parallelism, `--fail-fast`, layered env composition (process → global → profile → recipe)
- `must-cache`: on-disk cache under `.mustfile/cache/`; mtime and SHA-256 hash strategies
- `must-recipe-shell`: generic shell recipe (`sh -c`) with mtime/hash caching and env passthrough
- `must-recipe-rust`: `rust-bin`, `rust-lib`, `rust-test` recipe types via `cargo`
- `must-recipe-go`: `go-bin`, `go-test` recipe types with GOOS/GOARCH cross-compile support
- `must-recipe-cc`: `c-bin`, `c-lib` recipe types (static and shared) via host or cross C compiler
- `must-toolchain`: target triple parsing, Rust/Go/C toolchain discovery, container cross-compilation (Docker/Podman)
- `must-import`: Makefile importer — line-by-line lexer, AST parser, translator, TOML writer, Markdown report
- `must-cli`: CLI entry point with subcommands `build`, `test`, `list`, `clean`, `explain`, `import`, `doctor`, `graph`
- Global flags: `--dry-run`, `-j`, `--fail-fast`, `--profile`, `--target`
- `must doctor`: environment health check for Rust, Go, C compiler, container runtime, and cache
- `must graph`: dependency graph output in text, Graphviz DOT, and Mermaid formats (`--format`)
- `must import`: converts Makefiles to `Mustfile.toml` with a diff report (`MUSTFILE_IMPORT_REPORT.md`)
- `must explain`: cache-key breakdown showing inputs, env vars, computed hash, and hit/miss status
- GitHub Actions CI workflow (fmt + clippy + test on ubuntu-latest and macos-latest)
- GitHub Actions release workflow (4-platform matrix: x86_64/aarch64 × Linux/macOS)
