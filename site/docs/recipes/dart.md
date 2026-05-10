# Dart Recipes

must provides two Dart recipe types: `dart-bin` and `dart-test`.

## `dart-bin` — Compile a binary

```toml
[recipe.build]
type    = "dart-bin"
package = "bin/main.dart"
```

Runs: `dart compile exe bin/main.dart`

Cache key includes: package, env vars.

## `dart-test` — Run tests

```toml
[recipe.test]
type    = "dart-test"
package = "test/"
deps    = ["build"]
```

Runs: `dart test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | bin | Dart entry point file |
| `package` | string | test | Test directory path |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Examples

```toml
[project]
name = "my-dart-project"

[recipe.build]
type    = "dart-bin"
package = "bin/main.dart"

[recipe.test]
type    = "dart-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Flutter Recipes](flutter.md) — Flutter build and test
