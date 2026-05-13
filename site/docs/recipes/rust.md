# Rust Recipes

must provides three Rust recipe types: `rust-bin`, `rust-lib`, and `rust-test`.

## Quick start

Project structure:

```
myapp/
├── Cargo.toml
├── Mustfile.toml
└── src/
    └── main.rs
```

`Cargo.toml`:

```toml
[package]
name = "myapp"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "myapp"
path = "src/main.rs"
```

`src/main.rs`:

```rust
use std::env;

fn main() {
    let name = env::args().nth(1).unwrap_or_else(|| "world".into());
    println!("Hello, {name}!");
}
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"
version = "0.1.0"

[recipe.build]
type    = "rust-bin"
package = "myapp"

[recipe.test]
type    = "rust-test"
package = "myapp"
deps    = ["build"]

[recipe.lint]
type   = "shell"
script = "cargo clippy -- -D warnings"
deps   = ["build"]
```

Build and run:

```
$ must build
[build] running cargo build --release -p myapp
  Compiling myapp v0.1.0
  Finished release [optimized] target(s)

$ must test
[test] running cargo test -p myapp
  running 0 tests
  test result: ok

$ must lint
[lint] running cargo clippy -- -D warnings
  Finished dev [unoptimized + debuginfo]
```

## Try it

<div id="playground-rust" data-must-playground="rust"></div>

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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
