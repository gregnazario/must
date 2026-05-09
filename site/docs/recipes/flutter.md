# Flutter Recipes

must provides two Flutter recipe types: `flutter-build` and `flutter-test`.

## `flutter-build` — Build a Flutter app

```toml
[recipe.build]
type    = "flutter-build"
package = "."
```

Runs: `flutter build <platform>` (platform is derived from target: `apk`, `ios`, `web`, `macos`, `windows`, `linux`)

Cache key includes: `flutter --version`, package, target, env vars.

## `flutter-test` — Run tests

```toml
[recipe.test]
type    = "flutter-test"
package = "."
deps    = ["build"]
```

Runs: `flutter test` (in package directory)

Cache strategy: `never` (test results should always be fresh).

## Cross-platform builds

The build platform is determined by the target:

| Target | Platform |
|--------|----------|
| `android`, `android-arm`, `android-arm64` | `apk` |
| `ios` | `ios` |
| `web` | `web` |
| `macos` | `macos` |
| `windows` | `windows` |
| `linux` | `linux` |

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Project directory path (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"never"`), `phony` (always re-run), and `workdir` (working directory).

## Examples

```toml
[project]
name = "my-flutter-app"

[recipe.build]
type    = "flutter-build"
package = "."

[recipe.test]
type    = "flutter-test"
package = "."
deps    = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Dart Recipes](dart.md) — Dart build and test
