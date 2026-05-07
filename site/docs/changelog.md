# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] - 2026-05-05

### Added

#### New recipe types
- `ts-bin`, `ts-check`, `ts-lint`, `npm` for TypeScript/JavaScript
- `py-bin`, `py-test`, `py-lint` for Python (auto-detects uv vs pip)
- `zig-bin`, `zig-test` for Zig
- `docker-build`, `docker-push` (auto-detects docker vs podman)
- `java-bin`, `java-test` for Java via Gradle wrapper
- `kotlin-bin`, `kotlin-test` for Kotlin via Gradle wrapper
- `swift-bin`, `swift-test` for Swift
- `dotnet-build`, `dotnet-test`, `dotnet-publish` for .NET
- `ruby-bin`, `ruby-test` for Ruby (bundle + rspec)
- `dart-bin`, `dart-test` for Dart
- `elixir-build`, `elixir-test` for Elixir (mix)
- `flutter-build`, `flutter-test` for Flutter
- `nim-bin`, `nim-test` for Nim

#### New commands
- `must cache list/du/invalidate` — cache management
- `must outdated` — show cache status for all recipes
- `must init --template` — create from 7 templates
- `must watch` — rebuild on file changes
- `must completions` — shell completions
- `must log --follow` — stream logs in real time
- `must foreach` — run command in each recipe context

#### Features
- Output capture with live streaming
- Config validation per recipe type
- Env interpolation (`${VAR}`)
- Tool-not-found errors with install hints
- Command preview in `must explain`
- Lua plugins with 10-function stdlib
- Criterion benchmarks for cache, graph, config
- Coverage CI with toolchain installation

## [0.1.0] - 2026-04-29

### Added

- Core types: `Recipe` trait, `BuildContext`, `CacheKey`, `RecipeOutput`
- Config: TOML schema with validation and cycle detection
- Graph: DAG with topological sort and parallel waves
- Engine: async Tokio scheduler with fail-fast
- Cache: on-disk mtime and SHA-256 hash strategies
- Recipes: `shell`, `rust-bin/lib/test`, `go-bin/test`, `c-bin/lib`
- Import: Makefile importer with report
- CLI: `build`, `test`, `list`, `clean`, `explain`, `import`, `doctor`, `graph`
- GitHub Actions CI and release workflow
