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
