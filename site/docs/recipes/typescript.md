# TypeScript Recipes

must provides four TypeScript recipe types: `ts-bin`, `ts-check`, `ts-lint`, and `npm`.

## Quick start

Project layout:

```
myapp/
├── Mustfile.toml
├── package.json
├── tsconfig.json
└── src/
    ├── index.ts
    └── index.test.ts
```

`package.json`:

```json
{
  "name": "myapp",
  "scripts": {
    "build": "tsc",
    "test": "node dist/index.test.js"
  },
  "devDependencies": {
    "typescript": "^5.7.0"
  }
}
```

`tsconfig.json`:

```json
{
  "compilerOptions": {
    "outDir": "dist",
    "rootDir": "src",
    "strict": true,
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext"
  },
  "include": ["src"]
}
```

`src/index.ts`:

```typescript
export function greet(name: string): string {
  return `Hello, ${name}!`;
}

console.log(greet("world"));
```

`src/index.test.ts`:

```typescript
import { greet } from "./index.js";

function testGreet() {
  const result = greet("test");
  if (result !== "Hello, test!") {
    throw new Error(`expected "Hello, test!", got "${result}"`);
  }
  console.log("pass: greet");
}

testGreet();
```

`Mustfile.toml`:

```toml
[project]
name = "myapp"

[recipe.build]
type    = "ts-bin"
package = "tsconfig.json"
cache   = "hash"
inputs  = ["src/**/*.ts"]
outputs = ["dist/**/*.js"]

[recipe.check]
type    = "ts-check"
package = "tsconfig.json"

[recipe.test]
type       = "npm"
npm_script = "test"
deps       = ["build"]
```

Build and run:

```
$ must build
● build   compiling tsconfig.json … done (1.2s)
● test    running npm test … done (0.3s)

$ must run check
● check   type-checking tsconfig.json … done (0.4s)
```

## Try it

<div id="playground-typescript" data-must-playground="typescript"></div>

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

All recipes also support these common fields: `deps`, `env`, `cache` (`"hash"` / `"mtime"` / `"none"`), `phony` (always re-run).

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
