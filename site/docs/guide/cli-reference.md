# CLI Reference

## Global flags

| Flag | Description |
|------|-------------|
| `--file <PATH>` | Path to Mustfile.toml (default: search upward) |
| `--profile <NAME>` | Apply `[env.<profile>]` overrides (default: `default`) |
| `-j, --parallelism <N>` | Max parallel recipes (default: num_cpus) |
| `--dry-run` | Plan without executing |
| `--fail-fast` | Cancel in-flight recipes on first failure |
| `-v, -vv, -vvv` | Verbosity level |
| `--target <TARGET>` | Cross-compilation target or group name |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

## Commands

### `must build [RECIPES...]`

Build one or more recipes. Defaults to the `build` recipe.

```bash
must build                # build default "build" recipe
must build api frontend   # build specific recipes
must build --dry-run      # show what would run
must build --profile release
```

### `must run [RECIPES...]`

Alias for `must build`.

### `must test [RECIPES...]`

Run test recipes. Defaults to the `test` recipe.

```bash
must test
must test unit-test integration-test
```

### `must fmt`

Run the `fmt` recipe if defined.

### `must lint`

Run the `lint` recipe if defined.

### `must list`

List all recipes with their type and dependencies.

```bash
must list
```

Output:

```
NAME      TYPE       DEPS
build     rust-bin   —
test      rust-test  build
lint      shell      —
```

### `must graph`

Visualize the dependency graph.

```bash
must graph                # text output
must graph --format dot   # Graphviz DOT format
must graph --format mermaid
```

### `must explain <RECIPE>`

Show detailed information about a recipe: cache key, inputs, outputs, environment, and command preview.

```bash
must explain build
```

Output:

```
Recipe:     build
Type:       rust-bin
Package:    my-app
Cache:      hash
Key:        74852f22d02e93ac...
Inputs:     src/main.rs, Cargo.toml
Outputs:    target/release/my-app
Command:    cargo build --release -p my-app
```

### `must outdated`

Show cache status for all recipes.

```bash
must outdated
```

Output:

```
✓ build     [fresh]   rust-bin    cached 4s ago
⚠ test      [stale]   rust-test   deps changed
✗ lint      [miss]    shell       never built
```

### `must diff`

Show what changed since the last build.

### `must watch [RECIPES...]`

Watch for file changes and rebuild automatically.

```bash
must watch
must watch build test
```

### `must log [RECIPE]`

Show build log output.

```bash
must log build           # show last build log
must log build --follow  # stream in real time
must log --clear         # clear all logs
```

### `must cache <ACTION>`

Manage the build cache.

```bash
must cache list                    # list cached recipes
must cache du                      # show cache disk usage
must cache invalidate <RECIPE>     # invalidate a recipe's cache
must cache invalidate --all        # invalidate everything
```

### `must plugin <ACTION>`

Manage Lua plugins.

```bash
must plugin list                   # list discovered plugins
must plugin check <NAME>           # validate a plugin
must plugin install <URL>          # install from URL
must plugin remove <NAME>          # remove a plugin
```

### `must import <PATH>`

Import a Makefile and generate a starter Mustfile.toml.

```bash
must import Makefile
```

### `must init`

Initialize a new Mustfile.toml interactively.

### `must clean [--cache]`

Remove build outputs. With `--cache`, also clears the build cache.

```bash
must clean           # remove build outputs
must clean --cache   # remove outputs and cache
```

### `must doctor`

Check toolchains and environment.

```bash
must doctor
```

Output:

```
✓ rustc 1.85.0
✓ go 1.23.0
✓ node 22.0.0
✗ zig not found
```

### `must foreach -- <COMMAND>`

Run a command in each recipe's context.

```bash
must foreach -- build
must foreach --parallelism 4 --keep-going -- build
```

### `must completions <SHELL>`

Generate shell completions.

```bash
must completions bash
must completions zsh
must completions fish
```

## Environment variables

| Variable | Description |
|----------|-------------|
| `MUST_ARGS` | Extra arguments appended to every must invocation |
| `MUST_FILE` | Default Mustfile.toml path (same as `--file`) |
| `MUST_PROFILE` | Default profile (same as `--profile`) |
