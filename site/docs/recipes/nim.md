# Nim Recipes

must provides two Nim recipe types: `nim-bin` and `nim-test`.

## `nim-bin` — Build a binary

```toml
[recipe.build]
type    = "nim-bin"
package = "src/main.nim"
```

Runs: `nim c -d:release src/main.nim` (in package directory)

Cache key includes: `nim --version`, package, env vars.

## `nim-test` — Run tests

```toml
[recipe.test]
type    = "nim-test"
package = "tests/test_all.nim"
deps    = ["build"]
```

Runs: `nim r --hints:off tests/test_all.nim` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Source file path |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

## Examples

```toml
[project]
name = "my-nim-project"

[recipe.build]
type    = "nim-bin"
package = "src/main.nim"

[recipe.test]
type    = "nim-test"
package = "tests/test_all.nim"
deps    = ["build"]
```
