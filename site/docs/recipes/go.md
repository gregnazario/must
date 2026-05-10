# Go Recipes

must provides two Go recipe types: `go-bin` and `go-test`.

## `go-bin` — Build a binary

```toml
[recipe.build]
type       = "go-bin"
package    = "./cmd/server"
ldflags    = "-s -w"
build_tags = ["integration", "cgo"]
```

Runs: `go build [-tags <t>] [-ldflags <f>] <package>`

Cache key includes: `go version`, package name, ldflags, build tags, target, env vars.

## `go-test` — Run tests

```toml
[recipe.test]
type    = "go-test"
package = "./..."
deps    = ["build"]
```

Runs: `go test ./...`

Cache strategy: `never` (test results should always be fresh).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | all | Go package path |
| `ldflags` | string | bin | Linker flags (e.g. `-s -w`) |
| `build_tags` | string[] | bin | Build tags |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

## Cross-compilation

Go cross-compiles natively via `GOOS`/`GOARCH` environment variables — no extra toolchain needed. must sets these automatically based on the target triple.

## Examples

```toml
[project]
name = "my-go-service"

[recipe.server]
type       = "go-bin"
package    = "./cmd/server"
ldflags    = "-s -w"
build_tags = ["prod"]

[recipe.test]
type    = "go-test"
package = "./..."
deps    = ["server"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Cross-Compilation](../guide/cross-compilation.md) — GOOS/GOARCH auto-detection
- [Config Reference](../guide/config-reference.md) — full field reference
