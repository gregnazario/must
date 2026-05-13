# Shell Recipes

The `shell` type is the universal recipe — runs any shell command.

## Quick start

Create a project with shell recipes for building, testing, and cleaning a C program:

```toml
# Mustfile.toml
[project]
name = "hello"
version = "0.1.0"

[recipe.build]
type    = "shell"
inputs  = ["src/hello.c"]
outputs = ["build/hello"]
script  = "mkdir -p build && gcc -o build/hello src/hello.c"

[recipe.test]
type   = "shell"
deps   = ["build"]
phony  = true
script = "test \"$(./build/hello)\" = \"hello\""

[recipe.clean]
type   = "shell"
phony  = true
script = "rm -rf build"
```

```c
/* src/hello.c */
#include <stdio.h>

int main(void) {
    printf("hello\n");
    return 0;
}
```

```bash
$ must build test
✓ [#################] 1/1 (starting...)
  ✓ build
  ✓ test
2 built, 0 cached, 0 failed — 120ms
```

## Try it

<div id="playground-shell" data-must-playground="getting-started"></div>

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
| `phony` | bool | Always re-run |
| `cache` | `"hash"` / `"mtime"` / `"none"` | Caching strategy |
| `script_win` | string | Windows override for `script` |
| `scripts` | map | Per-OS script overrides (e.g., `macos`, `linux`, `linux.ubuntu`, `win`) |

## Working directory

Shell commands execute in the project root directory (where `Mustfile.toml` is located), regardless of where you run `must` from.

## Glob expansion

`inputs` supports glob patterns:

```toml
inputs = ["src/**/*.rs", "Cargo.toml"]
```

## Cross-platform

must uses `sh -c` on Unix and `cmd /C` on Windows for shell recipe execution. Use `script_win` to provide a Windows-specific override:

```toml
[recipe.clean]
type   = "shell"
phony  = true
script     = "rm -rf build"
script_win = "rmdir /s /q build"
```

See [Cross-Platform Scripts](../guide/cross-platform.md) for the full guide.

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

## See also

- [Cross-Platform Scripts](../guide/cross-platform.md) — `script_win` overrides for Windows
- [Caching](../guide/caching.md) — hash vs mtime strategies
- [Config Reference](../guide/config-reference.md) — full field reference
- [Plugin Recipes](../guide/plugins.md) — custom recipe types in Lua
