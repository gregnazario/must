# Zig Recipes

must provides two Zig recipe types: `zig-bin` and `zig-test`.

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
