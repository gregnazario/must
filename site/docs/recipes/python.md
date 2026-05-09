# Python Recipes

must provides three Python recipe types: `py-bin`, `py-test`, and `py-lint`.

## `py-bin` — Install dependencies

```toml
[recipe.build]
type    = "py-bin"
package = "."
```

Runs: `uv pip install .` (or `pip install .` if uv is not available)

Cache key includes: package, env vars.

## `py-test` — Run tests

```toml
[recipe.test]
type    = "py-test"
package = "."
deps    = ["build"]
```

Runs: `pytest` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## `py-lint` — Lint with ruff and mypy

```toml
[recipe.lint]
type    = "py-lint"
package = "src/"
```

Runs: `ruff check . && mypy .` (in package directory)

Cache strategy: `never` (lint results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Package directory path |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"never"`), `phony` (always re-run), and `workdir` (working directory).

## Examples

```toml
[project]
name = "my-python-project"

[recipe.build]
type    = "py-bin"
package = "."

[recipe.test]
type    = "py-test"
package = "."
deps    = ["build"]

[recipe.lint]
type    = "py-lint"
package = "src/"
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
