# must — User Guide

`must` is a polyglot build orchestrator. It reads a `Mustfile.toml`, resolves recipe dependencies, and executes recipes in parallel using mtime or hash-based caching.

## Installation

```bash
cargo install --locked mustfile
```

Or download a prebuilt binary from the [releases page](https://github.com/aptoslabs/mustfile/releases).

## Quick start

Create a `Mustfile.toml` in your project root:

```toml
[project]
name = "my-project"

[recipe.build]
type = "shell"
script = "gcc -o app main.c"
inputs = ["main.c"]
outputs = ["app"]

[recipe.test]
type = "shell"
deps = ["build"]
script = "./app --test"
```

Then run:

```bash
must build    # builds the 'build' recipe
must test     # builds 'test' and all its dependencies
must list     # shows all recipes
```

## Mustfile.toml reference

### `[project]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Project name |
| `version` | string | no | Version string |

### `[env.global]`

Key-value pairs injected into every recipe's environment:

```toml
[env.global]
CC = "gcc"
CFLAGS = "-Wall -O2"
```

Profile-specific overrides go under `[env.<profile>]`:

```toml
[env.release]
CARGO_PROFILE = "release"
```

Apply with `must build --profile release`.

### `[recipe.<name>]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `type` | string | required | `shell`, `rust-bin`, `rust-lib`, `rust-test`, `go-bin`, `go-test`, `c-bin`, `c-lib`, `ts-bin`, `ts-check`, `ts-lint`, `npm` |
| `deps` | list | `[]` | Recipe names that must complete first |
| `inputs` | list | `[]` | Glob patterns for input files (mtime tracking) |
| `outputs` | list | `[]` | Glob patterns for output files (mtime tracking) |
| `script` | string | — | Shell script (`type = "shell"` only) |
| `cache` | string | type default | `"mtime"`, `"hash"`, or `"none"` |
| `phony` | bool | `false` | Always re-run even if outputs are up to date |
| `env` | table | `{}` | Extra env vars for this recipe only |
| `package` | string | — | Package/project path (Rust/Go/TypeScript recipes) |
| `features` | list | `[]` | Cargo features (`rust-*` recipes) |
| `ldflags` | string | — | Linker flags (`go-bin` only) |
| `sources` | list | `[]` | Source files (`c-bin`, `c-lib`) |
| `includes` | list | `[]` | Include directories (`c-bin`, `c-lib`) |
| `link_libs` | list | `[]` | Libraries to link (`c-bin`, `c-lib`) |

### `[targets]`

Named groups of cross-compile targets:

```toml
[targets]
linux = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
apple = ["x86_64-apple-darwin", "aarch64-apple-darwin"]
```

## CLI reference

### `must build [recipes...]`

Build the named recipes (default: `build`) and all their dependencies.

```bash
must build                              # run 'build' recipe
must build codegen proto                # run 'codegen' and 'proto' recipes
must build --target aarch64-unknown-linux-gnu  # cross-compile
must build --target linux               # all targets in the 'linux' group
must build -j 8                         # 8 parallel workers
must build --dry-run                    # plan without executing
must build --fail-fast                  # stop on first failure
```

### `must test [recipes...]`

Same as `build` but defaults to the `test` recipe.

### `must list`

Print all recipes with their type and dependencies.

### `must explain <recipe>`

Show why a recipe will or won't rebuild: cache strategy, input files with hashes, env vars affecting the key, computed cache key, and current hit/miss status.

```bash
must explain build
```

### `must import`

Convert a Makefile to a `Mustfile.toml` starter.

```bash
must import                               # reads ./Makefile, writes ./Mustfile.toml
must import --makefile path/to/GNUmakefile --out path/to/Mustfile.toml
```

Writes a `MUSTFILE_IMPORT_REPORT.md` alongside the output listing what was translated, what needs manual attention (pattern rules, includes), and what was skipped.

### `must doctor`

Check whether required tools are installed and the cache is healthy.

```bash
must doctor
```

Checks: Rust/cargo (required), Go (optional), C compiler (optional), container runtime (optional), cache size.

### `must graph`

Print the recipe dependency graph.

```bash
must graph                    # human-readable text
must graph --format dot       # Graphviz DOT — pipe to: dot -Tpng -o graph.png
must graph --format mermaid   # Mermaid diagram for GitHub/GitLab markdown
```

### `must clean`

Remove build outputs and optionally the cache.

```bash
must clean           # clean declared outputs
must clean --cache   # also wipe .mustfile/cache/
```

## Caching

By default:
- **Shell recipes** use **mtime** caching: rebuild when any input file is newer than any output file.
- **First-class recipes** (Rust, Go, C) use **hash** caching: rebuild when the SHA-256 hash of inputs + env + flags changes.

Override per recipe:

```toml
[recipe.codegen]
type = "shell"
cache = "hash"
script = "python gen.py"
inputs = ["schema.proto"]
outputs = ["gen/"]
```

Set `cache = "none"` to always re-run.

## Cross-compilation

Add `--target <triple>` to any build command:

```bash
must build --target aarch64-unknown-linux-gnu
```

Or define and use a named group:

```toml
[targets]
linux = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]
```

```bash
must build --target linux
```

**Per-language behaviour:**
- **Rust**: uses `cargo` with `CARGO_TARGET_<TRIPLE>_LINKER` when a cross-linker is found.
- **Go**: sets `GOOS` and `GOARCH` derived from the triple.
- **C**: looks for `<triple>-gcc` or `<triple>-clang` in PATH.

## Profiles

Add per-profile env overrides and select with `--profile`:

```toml
[env.release]
EXTRA_FLAGS = "--optimize"
```

```bash
must build --profile release
```
