# Elixir Recipes

must provides two Elixir recipe types: `elixir-build` and `elixir-test`.

## Quick start

```
myapp/
├── Mustfile.toml
├── mix.exs
├── lib/
│   └── myapp.ex
└── test/
    └── myapp_test.exs
```

`mix.exs`:

```elixir
defmodule Myapp.MixProject do
  use Mix.Project

  def project do
    [
      app: :myapp,
      version: "0.1.0",
      elixir: "~> 1.16",
      deps: []
    ]
  end
end
```

`lib/myapp.ex`:

```elixir
defmodule Myapp do
  def greet do
    "Hello from Myapp!"
  end
end
```

`test/myapp_test.exs`:

```elixir
defmodule MyappTest do
  use ExUnit.Case

  test "greet returns hello message" do
    assert Myapp.greet() == "Hello from Myapp!"
  end
end
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"
version = "0.1.0"

[recipe.build]
type    = "elixir-build"
package = "."

[recipe.test]
type    = "elixir-test"
package = "."
deps    = ["build"]
```

Build and test:

```
$ must build
● build  mix deps.get && mix compile
  Resolving Hex dependencies...
  Dependency resolution completed:
  All dependencies up to date
  Compiling 1 file (.ex)
  ✓ build  (1.2s)

$ must test
● test   mix test
  .

  Finished in 0.03 seconds
  1 test, 0 failures
  ✓ test   (0.8s)
```

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
