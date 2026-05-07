# Shell Recipes

The `shell` type is the universal recipe — runs any shell command.

## Basic usage

```toml
[recipe.build]
type   = "shell"
script = "gcc -o app main.c"
inputs = ["main.c"]
outputs = ["app"]
```

## Caching

Shell recipes default to **mtime** caching — re-run when input files change.

### Opt into content-hash caching

```toml
[recipe.proto]
type   = "shell"
cache  = "hash"
inputs = ["proto/**/*.proto"]
script = "protoc --rust_out=src proto/*.proto"
```

### Always re-run (phony)

```toml
[recipe.clean]
type   = "shell"
phony  = true
script = "cargo clean && rm -rf dist/"
```

## All fields

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"shell"` | **Required** |
| `script` | string | Shell command to execute |
| `inputs` | string[] | Input file globs (for cache hashing) |
| `outputs` | string[] | Output file paths |
| `deps` | string[] | Dependencies |
| `env` | map | Environment variables |
| `workdir` | string | Working directory |
| `phony` | bool | Always re-run |
| `cache` | `"hash"` / `"mtime"` / `"never"` | Caching strategy |

## Glob expansion

`inputs` supports glob patterns:

```toml
inputs = ["src/**/*.rs", "Cargo.toml"]
```

## Cross-platform

must uses `sh -c` on Unix and `cmd /C` on Windows for shell recipe execution. Scripts should be compatible with both, or use `workdir` and conditional logic.

## Examples

```toml
[recipe.build]
type    = "shell"
inputs  = ["src/**/*.c"]
outputs = ["build/libcore.a"]
script  = "make -C src all"

[release.clean]
type   = "shell"
phony  = true
script = "rm -rf build/ dist/"

[recipe.integration]
type   = "shell"
deps   = ["build"]
phony  = true
env    = { SLOW_TESTS = "1" }
script = "./scripts/integration-test.sh"
```
