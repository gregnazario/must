# Getting Started

## Installation

=== "curl (macOS / Linux)"

    ```bash
    curl -fsSL https://github.com/anomalyco/mustfile/releases/latest/download/install.sh | sh
    ```

=== "cargo"

    ```bash
    cargo install --locked mustfile
    ```

=== "Homebrew"

    ```bash
    brew tap anomalyco/tap
    brew install must
    ```

=== "Windows"

    Download `must-x86_64-pc-windows-msvc.zip` from the [latest release](https://github.com/anomalyco/mustfile/releases/latest).

=== "From source"

    ```bash
    git clone https://github.com/anomalyco/mustfile.git
    cd mustfile
    cargo install --path crates/must-cli
    ```

## Verify

```bash
must --version
```

## Create your first Mustfile

Initialize a starter config:

```bash
must init
```

This creates a `Mustfile.toml` with a shell-based build recipe:

```toml
[project]
name = "my-project"

[recipe.build]
type   = "shell"
script = "echo 'Hello from must!'"
```

## Try it in the browser

<div id="playground-intro" data-must-playground="getting-started"></div>

## Build

```bash
must build
```

Output:

```
Hello from must!
1 built, 0 cached, 0 failed
```

## Add dependencies

```toml
[project]
name = "my-project"

[recipe.build]
type    = "rust-bin"
package = "my-app"

[recipe.test]
type    = "rust-test"
package = "my-app"
deps    = ["build"]
phony   = true

[recipe.lint]
type   = "shell"
script = "cargo clippy -- -D warnings"
phony  = true

[recipe.ci]
type   = "shell"
deps   = ["build", "test", "lint"]
phony  = true
script = "echo 'All checks passed'"
```

Now `must build test lint` runs all three, `must ci` runs the umbrella.

## Validate your config

<div id="validator-intro" data-must-validator></div>

## Check what's stale

```bash
must outdated
```

```
✓ build    [fresh]    rust-bin   cached 4s ago
⚠ test     [stale]    rust-test  deps changed
✗ lint     [miss]     shell      never built
```

## Explore

```bash
must list           # list all recipes
must graph          # show dependency graph
must explain build  # cache key, inputs, command preview
must doctor         # check toolchains
```

## Next steps

- [Config Reference](config-reference.md) — full Mustfile.toml schema
- [CLI Reference](cli-reference.md) — all commands and flags
- [Recipes](../recipes/index.md) — per-language guides
