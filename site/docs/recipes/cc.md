# C Recipes

must provides two C recipe types: `c-bin` and `c-lib`.

## `c-bin` — Build a binary

```toml
[recipe.build]
type     = "c-bin"
sources  = ["src/main.c", "src/util.c"]
includes = ["include"]
cflags   = ["-O2", "-Wall"]
link_libs = ["m"]
```

Runs: `cc src/main.c src/util.c -Iinclude -lm -O2 -Wall -o build/<name>`

Cache key includes: `cc --version`, sources, includes, link_libs, cflags, target, env vars.

## `c-lib` — Build a library

```toml
[recipe.mylib]
type       = "c-lib"
sources    = ["src/lib.c"]
includes   = ["include"]
static_lib = true
```

Runs (static): `cc -fPIC -c src/lib.c -o build/lib.o && ar rcs build/libmylib.a build/lib.o`

Runs (shared): `cc -shared -fPIC src/lib.c -o build/libmylib.so`

Cache key includes: `cc --version`, sources, includes, link_libs, cflags, lib type, target, env vars.

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `sources` | string[] | all | Source files |
| `includes` | string[] | all | Include directories |
| `cflags` | string[] | all | Extra compiler flags |
| `link_libs` | string[] | all | Libraries to link (`-l`) |
| `static_lib` | bool | lib | `true` = `.a`, `false` = `.so` (default: `true`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Cross-compilation

```toml
[recipe.cli]
type    = "c-bin"
sources = ["src/main.c"]

[recipe.cli.cross]
"aarch64-unknown-linux-gnu" = { linker = "aarch64-linux-gnu-gcc" }
"x86_64-unknown-linux-gnu" = { cross = "container" }
```

## Examples

```toml
[project]
name = "my-c-project"

[recipe.mybin]
type     = "c-bin"
sources  = ["src/main.c", "src/util.c"]
includes = ["include"]
cflags   = ["-O2"]

[recipe.mylib]
type       = "c-lib"
sources    = ["src/lib.c"]
includes   = ["include"]
static_lib = true
deps       = []

[recipe.mylib-shared]
type       = "c-lib"
sources    = ["src/lib.c"]
static_lib = false
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Cross-Compilation](../guide/cross-compilation.md) — cross-compilers and sysroots
- [Config Reference](../guide/config-reference.md) — full field reference
