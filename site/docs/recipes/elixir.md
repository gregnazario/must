# Elixir Recipes

must provides two Elixir recipe types: `elixir-build` and `elixir-test`.

## `elixir-build` — Compile a project

```toml
[recipe.build]
type    = "elixir-build"
package = "."
```

Runs: `mix deps.get && mix compile` (in package directory)

Cache key includes: `elixir --version`, package, env vars.

## `elixir-test` — Run tests

```toml
[recipe.test]
type    = "elixir-test"
package = "."
deps    = ["build"]
```

Runs: `mix test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Project directory path (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

## Examples

```toml
[project]
name = "my-elixir-umbrella"

[recipe.build-api]
type    = "elixir-build"
package = "apps/api"

[recipe.test-api]
type    = "elixir-test"
package = "apps/api"
deps    = ["build-api"]

[recipe.build-web]
type    = "elixir-build"
package = "apps/web"

[recipe.test-web]
type    = "elixir-test"
package = "apps/web"
deps    = ["build-web"]
```
