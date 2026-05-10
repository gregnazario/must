# Swift Recipes

must provides two Swift recipe types: `swift-bin` and `swift-test`.

## `swift-bin` — Build a binary

```toml
[recipe.build]
type    = "swift-bin"
package = "."
```

Runs: `swift build -c release` (in package directory)

Cache key includes: `swift --version`, package, env vars.

## `swift-test` — Run tests

```toml
[recipe.test]
type    = "swift-test"
package = "."
deps    = ["build"]
```

Runs: `swift test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Package directory path (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-swift-project"

[recipe.build]
type    = "swift-bin"
package = "."

[recipe.test]
type    = "swift-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
