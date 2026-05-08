# Mustfile

<p style="font-size: 1.2em; color: var(--md-typeset-a-color);">
One config. Every language.
</p>

A polyglot build orchestrator with first-class support for **42 recipe types** across **17+ languages**.

Replace your Makefiles, Justfiles, and Taskfiles with a single `Mustfile.toml` that knows how to build, test, cache, and cross-compile every language in your project.

## Quick start

```bash
# Install
curl -fsSL https://github.com/anomalyco/mustfile/releases/latest/download/install.sh | sh

# Or with cargo
cargo install --locked mustfile
```

Create a `Mustfile.toml`:

```toml
[project]
name = "my-app"

[recipe.build]
type    = "rust-bin"
package = "my-app"

[recipe.test]
type    = "rust-test"
package = "my-app"
deps    = ["build"]

[recipe.lint]
type   = "shell"
script = "cargo clippy -- -D warnings"
phony  = true
```

Run:

```bash
must build      # builds the 'build' recipe
must test       # runs test + dependencies
must outdated   # shows what's stale
must explain build  # cache key, inputs, command preview
```

## Why must?

| Problem | What must does |
|---------|---------------|
| mtime rebuilds break on branch switches | Content-hashed caching for compiled recipes |
| No language awareness — every build file reinvents toolchains | First-class recipe types that know each toolchain |
| Cross-compilation is manual and per-project | Automatic target resolution + container opt-in |
| Different commands per language (`npm run`, `cargo build`, `go build`) | Consistent verbs: `must build`, `must test`, `must lint` |
| Hard to introspect build state | `must list`, `must graph`, `must outdated`, `must explain` |

## Supported languages

| Language | Recipe types |
|----------|-------------|
| Shell | `shell` |
| Rust | `rust-bin`, `rust-lib`, `rust-test` |
| Go | `go-bin`, `go-test` |
| C/C++ | `c-bin`, `c-lib` |
| TypeScript | `ts-bin`, `ts-check`, `ts-lint`, `npm` |
| Python | `py-bin`, `py-test`, `py-lint` |
| Zig | `zig-bin`, `zig-test` |
| Java | `java-bin`, `java-test` |
| Kotlin | `kotlin-bin`, `kotlin-test` |
| Swift | `swift-bin`, `swift-test` |
| .NET | `dotnet-build`, `dotnet-test`, `dotnet-publish` |
| Ruby | `ruby-bin`, `ruby-test` |
| Dart | `dart-bin`, `dart-test` |
| Elixir | `elixir-build`, `elixir-test` |
| Flutter | `flutter-build`, `flutter-test` |
| Nim | `nim-bin`, `nim-test` |
| Docker | `docker-build`, `docker-push` |
| Precompiled | `precompiled-bin` |
| Lua plugins | `plugin` |

## Features

- :material-language-rust: **42 recipe types** with first-class caching
- :material-speedometer: **Smart caching** — content-hash for compiled, mtime for shell
- :material-earth: **Cross-compilation** — automatic toolchain resolution
- :material-graph: **Dependency graph** — `must graph` visualizes your build
- :material-eye: **Watch mode** — `must watch` rebuilds on file changes
- :material-puzzle: **Lua plugins** — extend with `.lua` scripts
- :material-github: **GitHub Actions** — drop-in action for CI
- :material-import: **Import** — convert Makefiles automatically

## Next steps

- [Getting Started](guide/getting-started.md) — install and first project
- [Config Reference](guide/config-reference.md) — full Mustfile.toml schema
- [CLI Reference](guide/cli-reference.md) — all commands
- [Recipes](recipes/index.md) — per-language recipe guides
