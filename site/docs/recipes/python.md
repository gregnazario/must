# Python Recipes

must provides three Python recipe types: `py-bin`, `py-test`, and `py-lint`.

## Try it

<div id="playground-python" data-must-playground="python"></div>

## Quick start

### Project structure

```
myapp/
├── pyproject.toml
├── Mustfile.toml
├── src/
│   └── myapp/
│       ├── __init__.py
│       └── main.py
└── tests/
    └── test_main.py
```

### pyproject.toml

```toml
[project]
name = "myapp"
version = "0.1.0"
requires-python = ">=3.10"
```

### src/myapp/__init__.py

```python
__version__ = "0.1.0"
```

### src/myapp/main.py

```python
def greet(name: str) -> str:
    return f"hello, {name}"


def main() -> None:
    print(greet("world"))


if __name__ == "__main__":
    main()
```

### tests/test_main.py

```python
from myapp.main import greet


def test_greet() -> None:
    assert greet("world") == "hello, world"
```

### Mustfile.toml

```toml
[project]
name = "myapp"

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

### Build, test, and lint

```
$ must build
● build  py-bin  myapp  done (1.2s)

$ must test
● build  py-bin    myapp  cached
● test   py-test   myapp  done (0.8s)

$ must lint
● build  py-bin    myapp  cached
● lint   py-lint   myapp  done (1.5s)
```

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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
