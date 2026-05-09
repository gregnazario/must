# TypeScript Recipes

must provides four TypeScript recipe types: `ts-bin`, `ts-check`, `ts-lint`, and `npm`.

## `ts-bin` — Compile TypeScript

```toml
[recipe.build]
type    = "ts-bin"
package = "tsconfig.json"
```

Runs: `tsc --project tsconfig.json`

Cache key includes: package, env vars.

## `ts-check` — Type-check only

```toml
[recipe.typecheck]
type    = "ts-check"
package = "tsconfig.json"
```

Runs: `tsc --noEmit --project tsconfig.json`

Cache strategy: `never` (type-check results should always be fresh).

## `ts-lint` — Lint with Biome

```toml
[recipe.lint]
type    = "ts-lint"
package = "src/"
```

Runs: `biome check src/`

Cache strategy: `never` (lint results should always be fresh).

## `npm` — Run an npm script

```toml
[recipe.bundle]
type       = "npm"
npm_script = "build"
workdir    = "packages/api"
```

Runs: `npm run build` (in `packages/api`)

Cache strategy: `never` (npm scripts are not cached).

## Fields

| Field | Type | Applies to | Description |
|-------|------|-----------|-------------|
| `package` | string | ts-bin, ts-check, ts-lint | tsconfig path or directory |
| `npm_script` | string | npm | npm script name to run |
| `workdir` | string | npm | Working directory (default `.`) |
| `deps` | string[] | all | Dependencies |
| `env` | map | all | Environment variables |

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"never"`), `phony` (always re-run), and `workdir` (working directory).

## Examples

```toml
[project]
name = "my-ts-workspace"

[recipe.build]
type    = "ts-bin"
package = "tsconfig.json"

[recipe.typecheck]
type    = "ts-check"
package = "tsconfig.json"
deps    = ["build"]

[recipe.lint]
type    = "ts-lint"
package = "src/"

[recipe.bundle-api]
type       = "npm"
npm_script = "build"
workdir    = "packages/api"
deps       = ["build"]
```

## See also

- [Caching](../guide/caching.md) — how cache keys are computed
- [Config Reference](../guide/config-reference.md) — full field reference
- [Shell Recipes](shell.md) — custom build scripts
