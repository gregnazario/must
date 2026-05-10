# Mustfile

> A polyglot build orchestrator with first-class support for Rust, Go, C/C++, TypeScript, Python, Zig, and Docker. One binary, one config, consistent verbs across languages.

Mustfile sits between pure task runners (Make, Just) and full build systems (Bazel, Buck2):

- **Consistent verbs:** `must build`, `must test`, `must outdated` — same commands regardless of language
- **40 recipe types:** First-class support for Rust, Go, C/C++, TypeScript, Python, Zig, Docker, Java, Kotlin, Swift, .NET, Ruby, Dart, Elixir, Flutter, Nim, precompiled binaries, shell, and Lua plugins
- **Pragmatic caching:** Content-hash caching for compiled recipes; mtime for shell; `must cache` for management
- **Cross-compilation:** Automatic GOOS/GOARCH, cross-rs containers, and C cross-compilers
- **Env interpolation:** `${VAR}` expansion in scripts, images, and flags from layered env config
- **Lua plugins:** Extend must with `.lua` files — shell exec, file I/O, glob, env access
- **Build logs:** `must log <recipe>` shows last build output; `must log --follow` streams in real time
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
| `shell` | Any | Generic `sh -c` / `cmd /C` script with mtime/hash caching |
| `rust-bin` | Rust | `cargo build -p <package>` |
| `rust-lib` | Rust | `cargo build --lib -p <package>` |
| `rust-test` | Rust | `cargo test -p <package>` |
| `go-bin` | Go | `go build <package>` with optional ldflags |
| `go-test` | Go | `go test <package>` |
| `c-bin` | C | Compile and link a binary |
| `c-lib` | C | Build static or shared library |
| `ts-bin` | TypeScript | TypeScript compilation |
| `ts-check` | TypeScript | `tsc --noEmit` type checking |
| `ts-lint` | TypeScript | ESLint / Biome |
| `npm` | JS/TS | Run npm scripts |
| `py-bin` | Python | Build Python package (uv or pip) |
| `py-test` | Python | Run pytest |
| `py-lint` | Python | Ruff + mypy |
| `zig-bin` | Zig | `zig build` |
| `zig-test` | Zig | `zig build test` |
| `docker-build` | Docker | `docker build` with tag and build args |
| `docker-push` | Docker | `docker push` |
| `java-bin` | Java | Gradle build |
| `java-test` | Java | Gradle test |
| `kotlin-bin` | Kotlin | Gradle build |
| `kotlin-test` | Kotlin | Gradle test |
| `swift-bin` | Swift | `swift build` |
| `swift-test` | Swift | `swift test` |
| `dotnet-build` | .NET | `dotnet build` |
| `dotnet-test` | .NET | `dotnet test` |
| `dotnet-publish` | .NET | `dotnet publish` |
| `ruby-bin` | Ruby | Bundle install + exec |
| `ruby-test` | Ruby | Bundle exec rake test |
| `dart-bin` | Dart | `dart compile exe` |
| `dart-test` | Dart | `dart test` |
| `elixir-build` | Elixir | `mix deps.get` + `mix compile` |
| `elixir-test` | Elixir | `mix test` |
| `flutter-build` | Flutter | `flutter build` (multi-platform) |
| `flutter-test` | Flutter | `flutter test` |
| `nim-bin` | Nim | `nim c -d:release` |
| `nim-test` | Nim | `nim r --hints:off` |
| `precompiled-bin` | Any | Download and cache prebuilt binaries with SHA-256 verification |
| `plugin` | Lua | User-defined recipe via Lua script |

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
| `must log <recipe>` | Show last build output for a recipe |
| `must log` | List all recipes with stored logs and sizes |
| `must log --follow <recipe>` | Stream log output in real time |
| `must log --clear` | Clear all stored logs |
| `must diff [revision]` | Diff build manifests between runs |
| `must foreach <command>` | Run a command in each recipe directory |
| `must plugin list` | List discovered plugins with validation status |
| `must plugin check <name>` | Validate a plugin without executing it |
| `must plugin install <url>` | Install a plugin from a URL |
| `must clean [--cache]` | Remove outputs and optionally cache |
| `must fmt` | Check code formatting |
| `must lint` | Run linters |
| `must init [--template]` | Create a Mustfile.toml from a template |
| `must import` | Convert a Makefile to Mustfile.toml |
| `must watch [recipes]` | Watch files and rebuild on change |
| `must doctor` | Check toolchains and environment |
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
- **None:** Always re-run (tests, lints)

```toml
[recipe.codegen]
type    = "shell"
cache   = "hash"
inputs  = ["proto/*.proto"]
outputs = ["src/generated.rs"]
script  = "protoc --rust_out=src proto/*.proto"
```

## Lua Plugins

Extend must with custom recipe types written in Lua. Place `.lua` files in `.mustfile/plugins/`:

```lua
-- .mustfile/plugins/protoc.lua
function execute(ctx)
    local protos = glob("proto/*.proto")
    for _, p in ipairs(protos) do
        shell_exec("protoc --rust_out=src " .. p)
    end
    return { stdout = "generated " .. #protos .. " files", stderr = "", success = true }
end
```

Reference it in your config:

```toml
[recipe.codegen]
type   = "plugin"
plugin = "protoc"
deps   = ["setup"]
```

### Plugin API

Every plugin receives a `ctx` table with:

| Field | Type | Description |
|-------|------|-------------|
| `ctx.project_root` | string | Project root directory |
| `ctx.cache_dir` | string | Cache directory path |
| `ctx.target` | string | Build target triple |
| `ctx.profile` | string | Active profile |
| `ctx.dry_run` | boolean | Whether dry-run is active |
| `ctx.env` | table | Environment variables |

### Built-in Functions

Plugins have access to a standard library:

| Function | Description |
|----------|-------------|
| `shell_exec(cmd)` | Run a shell command, returns `{success, exit_code, stdout, stderr}` |
| `read_file(path)` | Read file contents as string |
| `write_file(path, content)` | Write string to file |
| `file_exists(path)` | Check if file or directory exists |
| `mkdir(path)` | Create directory (recursive) |
| `glob(pattern)` | Return matching file paths as array |
| `env_get(key)` | Get environment variable |
| `set_env(key, value)` | Set environment variable |
| `log_info(msg)` | Log info message |
| `log_warn(msg)` | Log warning message |

### Optional Hooks

Plugins can define optional functions:

```lua
deps = {"codegen", "compile"}

function inputs(ctx)
    return { "src/main.lua", "config.toml" }
end

function outputs(ctx)
    return { "dist/app.js" }
end

function cache_key(ctx)
    return "custom-key-" .. ctx.profile
end
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
- `cross-platform/` — Platform-specific scripts with `script_win` overrides
- `precompiled-tools/` — Download and cache prebuilt binaries
- `ci-cd/` — Full CI/CD workflow example
- `plugin-project/` — Custom Lua plugin recipes
- `release-workflow/` — Multi-target release automation

Must itself is built with must — see the root [`Mustfile.toml`](Mustfile.toml) for a real-world example.

## Documentation

- [Doc Site](https://mustfile.ai) — full guides, recipe reference, and migration docs
- [Architecture](docs/DESIGN.md) — execution model and internals
- [Migration](docs/MIGRATION_GUIDE.md) — migrating from Make, Just, and Taskfile

## License

MIT — see [LICENSE](LICENSE).
