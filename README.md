# Mustfile

> A polyglot build orchestrator with first-class support for Rust, Go, C/C++, TypeScript, Python, Zig, and Docker. One binary, one config, consistent verbs across languages.

Mustfile sits between pure task runners (Make, Just) and full build systems (Bazel, Buck2):

- **Consistent verbs:** `must build`, `must test`, `must outdated` — same commands regardless of language
- **19 recipe types:** First-class support for Rust, Go, C/C++, TypeScript, Python, Zig, Docker, and shell
- **Pragmatic caching:** Content-hash caching for compiled recipes; mtime for shell; `must cache` for management
- **Cross-compilation:** Automatic GOOS/GOARCH, cross-rs containers, and C cross-compilers
- **Env interpolation:** `${VAR}` expansion in scripts, images, and flags from layered env config
- **Migration-friendly:** `must import` converts existing Makefiles

## Install

```bash
cargo install --path crates/must-cli
```

Or download a prebuilt binary from [Releases](https://github.com/gregnazario/mustfile/releases).

## Quick Start

```bash
# Create a new project
must init --name myapp --template rust

# Build
must build

# Run tests
must test

# See what will happen
must explain build

# Check cache freshness
must outdated

# Watch for changes and rebuild
must watch
```

## Mustfile.toml

A `Mustfile.toml` at the project root defines your build graph:

```toml
[project]
name = "myapp"
version = "0.1.0"

[env]
RUST_LOG = "info"

[env.release]
RUST_LOG = "warn"

[recipe.build]
type    = "rust-bin"
package = "myapp"

[recipe.test]
type    = "rust-test"
package = "myapp"
deps    = ["build"]

[recipe.lint]
type   = "shell"
phony  = true
script = "cargo clippy -- -D warnings"

[recipe.ci]
type   = "shell"
deps   = ["lint", "test"]
phony  = true
script = "echo CI passed"
```

## Recipe Types

| Type | Language | Description |
|------|----------|-------------|
| `shell` | Any | Generic `sh -c` script with mtime/hash caching |
| `rust-bin` | Rust | `cargo build -p <package>` |
| `rust-lib` | Rust | `cargo build --lib -p <package>` |
| `rust-test` | Rust | `cargo test -p <package>` |
| `go-bin` | Go | `go build <package>` with optional ldflags |
| `go-test` | Go | `go test <package>` |
| `c-bin` | C | Compile and link a binary |
| `c-lib` | C | Build static or shared library |
| `ts-bin` | TypeScript | TypeScript compilation |
| `ts-check` | TypeScript | `tsc --noEmit` type checking |
| `ts-lint` | TypeScript | ESLint |
| `npm` | JS/TS | Run npm scripts |
| `py-bin` | Python | Build Python package (uv or pip) |
| `py-test` | Python | Run pytest |
| `py-lint` | Python | Ruff + mypy |
| `zig-bin` | Zig | `zig build` |
| `zig-test` | Zig | `zig build test` |
| `docker-build` | Docker | `docker build` with tag and build args |
| `docker-push` | Docker | `docker push` |

## Commands

| Command | Description |
|---------|-------------|
| `must build [recipes]` | Build one or more recipes |
| `must test [recipes]` | Run test recipes |
| `must run <recipes>` | Alias for build |
| `must explain <recipe>` | Show cache key, inputs, env, and command preview |
| `must list` | List all recipes with types and deps |
| `must graph [--format text\|dot\|mermaid]` | Visualize the dependency graph |
| `must outdated` | Show cache status for all recipes |
| `must cache list` | List cached entries |
| `must cache invalidate <recipe>` | Clear cache for a recipe |
| `must cache invalidate --all` | Clear all cache |
| `must cache du` | Show cache disk usage |
| `must clean [--cache]` | Remove outputs and optionally cache |
| `must init [--template]` | Create a Mustfile.toml from a template |
| `must watch [recipes]` | Watch files and rebuild on change |
| `must doctor` | Check toolchains and environment |
| `must import` | Convert a Makefile to Mustfile.toml |
| `must completions <shell>` | Generate shell completions |

## Environment Variables

Env vars are layered (highest priority wins):

1. Process environment
2. Global `[env]` in config
3. Profile `[env.release]`
4. Per-recipe `[recipe.build.env]`
5. Toolchain env (cross-compilation)

Use `${VAR}` in recipe fields for interpolation:

```toml
[env]
REGISTRY = "ghcr.io/myorg"

[recipe.api-image]
type  = "docker-build"
image = "${REGISTRY}/api:latest"
```

## Cross-Compilation

```toml
[targets]
release = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
]

[recipe.build.cross]
"aarch64-unknown-linux-gnu" = { linker = "aarch64-linux-gnu-gcc", cross = "container" }
```

```bash
must build --target release
```

## Caching

- **Hash strategy:** SHA-256 of inputs + env — branch-switch safe
- **Mtime strategy:** Compare input/output timestamps — fast
- **Never:** Always re-run (tests, lints)

```toml
[recipe.codegen]
type    = "shell"
cache   = "hash"
inputs  = ["proto/*.proto"]
outputs = ["src/generated.rs"]
script  = "protoc --rust_out=src proto/*.proto"
```

## Templates

`must init` supports 7 templates: `minimal`, `rust`, `go`, `python`, `zig`, `docker`, `polyglot`.

## Examples

See the [`examples/`](examples/) directory for complete project setups:

- `simple/` — Shell recipes with mtime and hash caching
- `rust-app/` — Rust binary with cross-compilation and release profiles
- `polyglot/` — Rust CLI + Go server + C library + shell glue
- `python-service/` — Python package with py-bin, py-test, py-lint
- `ts-monorepo/` — TypeScript monorepo with ts-bin, ts-check, npm
- `docker-monorepo/` — Multi-service Docker builds with env interpolation
- `zig-tool/` — Zig binary and test recipes

## Documentation

- [User Guide](docs/USER_GUIDE.md) — Complete usage reference
- [Migration Guide](docs/MIGRATION_GUIDE.md) — Migrating from Make
- [Design](docs/DESIGN.md) — Architecture and execution model

## License

MIT — see [LICENSE](LICENSE).
