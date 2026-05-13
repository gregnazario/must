# Zig Recipes

must provides two Zig recipe types: `zig-bin` and `zig-test`.

## Quick start

Project structure:

```
myapp/
├── build.zig
├── build.zig.zon
├── Mustfile.toml
└── src/
    └── main.zig
```

`build.zig.zon`:

```toml
.{
    .name = "myapp",
    .version = "0.1.0",
    .dependencies = .{},
}
```

`src/main.zig`:

```zig
const std = @import("std");

pub fn main() !void {
    std.debug.print("Hello from {s}!\n", .{"myapp"});
}
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"
version = "0.1.0"

[recipe.build]
type    = "zig-bin"
package = "myapp"

[recipe.test]
type    = "zig-test"
package = "."
deps    = ["build"]
```

Build:

```
$ must build
[build] zig build myapp -Doptimize=ReleaseSafe
[build] done 1.2s
```

Test:

```
$ must test
[build] zig build myapp -Doptimize=ReleaseSafe
[build] cached
[test]  zig build test
[test]  All 1 tests passed.
[test]  done 0.4s
```

## `zig-bin` — Build a binary

```toml
[recipe.build]
type    = "zig-bin"
package = "myapp"
```

Runs: `zig build myapp -Doptimize=ReleaseSafe`

Cache key includes: package, env vars.

## `zig-test` — Run tests

```toml
[recipe.test]
type    = "zig-test"
package = "."
deps    = ["build"]
```

Runs: `zig build test`

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | bin | Build step name |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-zig-project"

[recipe.build]
type    = "zig-bin"
package = "myapp"

[recipe.test]
type    = "zig-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Cross-Compilation](../guide/cross-compilation.md) — Zig cross-compilation
- [Config Reference](../guide/config-reference.md) — full field reference
