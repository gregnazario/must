# Rust Recipes

must provides three Rust recipe types: `rust-bin`, `rust-lib`, and `rust-test`.

## `rust-bin` — Build a binary

```toml
[recipe.build]
type     = "rust-bin"
package  = "my-app"
features = ["cli"]
```

Runs: `cargo build --release -p my-app --features cli`

Cache key includes: `rustc --version`, package name, features, target, env vars.

## `rust-lib` — Build a library

```toml
[recipe.core]
type    = "rust-lib"
package = "my-core"
```

Runs: `cargo build --lib --release -p my-core`

## `rust-test` — Run tests

```toml
[recipe.test]
type    = "rust-test"
package = "my-app"
deps    = ["build"]
```

Runs: `cargo test -p my-app`

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Cargo package name |
| `features` | string[] | bin, lib | Feature flags |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |
| `workdir` | string | all | Working directory |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"never"`), `phony` (always re-run), and `workdir` (working directory).

## Cross-compilation

```toml
[recipe.cli]
type    = "rust-bin"
package = "cli"

[recipe.cli.cross]
"aarch64-unknown-linux-gnu" = { linker = "aarch64-linux-gnu-gcc" }
"x86_64-pc-windows-msvc" = { cross = "container" }
```

## Examples

```toml
[project]
name = "my-workspace"

[recipe.cli]
type     = "rust-bin"
package  = "my-cli"
features = ["default"]

[recipe.lib]
type    = "rust-lib"
package = "my-lib"

[recipe.test-cli]
type    = "rust-test"
package = "my-cli"
deps    = ["cli", "lib"]

[recipe.test-lib]
type    = "rust-test"
package = "my-lib"
deps    = ["lib"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Cross-Compilation](../guide/cross-compilation.md) — target triples and cross-rs
- [Config Reference](../guide/config-reference.md) — full field reference
