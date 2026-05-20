# Migration Guide: Makefile / Justfile / Taskfile / npm scripts → Mustfile

This guide helps you move from Make, [Just](https://github.com/casey/just), [Taskfile](https://taskfile.dev), or `package.json` scripts to Mustfile. Each section shows the same project expressed in every tool so you can see the differences directly.

---

## Why migrate?

| Pain point | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| Tab-significant syntax | Yes | No | No | No | No (TOML) |
| Language-aware builds | No | No | No | No | Yes (Rust, Go, C, TypeScript, Python, Zig, Docker) |
| Automatic cross-compilation | No | No | No | No | Yes (per-toolchain env + container opt-in) |
| Content-hash caching | No | No | No | No | Yes (first-class recipes) |
| Parallel DAG execution | `make -j` | `-j` flag | `RUN_IN_PARALLEL` | No | Automatic (wave-based) |
| Dependency graph visualization | No | No | `--dry` | No | `must graph` (text / DOT / Mermaid) |
| Cache introspection | No | No | No | No | `must explain <recipe>` |
| Polyglot in one file | Manual | Manual | Manual | No | First-class recipe types |
| Single static binary | Yes | Yes | No (Go) | No (Node.js) | Yes |

---

## Quick reference: recipe definitions

### Makefile

```makefile
.PHONY: build test lint ci

build:
	cargo build -p myapp

test: build
	cargo test -p myapp

lint:
	cargo clippy -- -D warnings

ci: lint test
	@echo CI passed
```

### Justfile

```just
build:
    cargo build -p myapp

test: build
    cargo test -p myapp

lint:
    cargo clippy -- -D warnings

ci: lint test
    echo CI passed
```

### Taskfile

```yaml
version: "3"
tasks:
  build:
    cmds: [cargo build -p myapp]
  test:
    deps: [build]
    cmds: [cargo test -p myapp]
  lint:
    cmds: [cargo clippy -- -D warnings]
  ci:
    deps: [lint, test]
    cmds: [echo CI passed]
```

### npm scripts

```json
{
  "name": "myapp",
  "scripts": {
    "build": "cargo build -p myapp",
    "test": "cargo test -p myapp",
    "lint": "cargo clippy -- -D warnings",
    "ci": "npm run lint && npm run test"
  }
}
```

Run with `npm run build`, `npm run test`, etc. No dependency graph — `ci` manually chains with `&&`.

### Mustfile

```toml
[project]
name = "myapp"

[recipe.build]
type    = "rust-bin"
package = "myapp"

[recipe.test]
type    = "rust-test"
package = "myapp"
deps    = ["build"]

[recipe.lint]
type   = "shell"
phony  = true
script = "cargo clippy -- -D warnings"

[recipe.ci]
type   = "shell"
deps   = ["lint", "test"]
phony  = true
script = "echo CI passed"
```

---

## Concept mapping

### Tasks / recipes

| Concept | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| A unit of work | target / rule | recipe | task | script | recipe |
| Run a task | `make build` | `just build` | `task build` | `npm run build` | `must build` |
| Default task | first target | first recipe | `default` task | `npm start` | `build` recipe |
| List all tasks | `make -q` | `just --list` | `task --list` | `npm run` | `must list` |
| Dry run | `make -n` | `just --dry-run` | `task --dry` | No | `must build --dry-run` |

### Dependencies

| Concept | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| Task depends on another | `target: dep` | `recipe: dep` | `deps: [dep]` | `&&` chaining | `deps = ["dep"]` |
| File dependency | `target: file.c` | N/A | `sources: [file.c]` | N/A | `inputs = ["file.c"]` |
| Order-only prerequisite | `target: \| dep` | N/A | N/A | N/A | `deps = ["dep"]` (same) |
| Run deps in parallel | `make -j` | N/A (sequential) | `RUN_IN_PARALLEL: true` | `npm-run-all -p` | Automatic |

### Environment variables

| Concept | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| Global env | `export VAR = val` | `export VAR := "val"` | `env: VAR: val` | `cross-env` | `[env] VAR = "val"` |
| Per-task env | `target: export VAR=val` | `recipe: export VAR=val` | `env: VAR: val` | Per-script prefix | `[recipe.name.env] VAR = "val"` |
| Profile/variant | Duplicate targets | Duplicate recipes | `vars:` + `includes` | Duplicate scripts | `[env.profile]` + `--profile` |
| `.env` file | Manual | `set dotenv-load` | `dotenv: true` | `dotenv` package | Not built-in (use `script`) |

### Caching

| Concept | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| Skip if up to date | mtime (file targets) | N/A (always runs) | `sources` + `generates` mtime | N/A (always runs) | `inputs`/`outputs` mtime or hash |
| Content-hash caching | No | No | No | No | `cache = "hash"` |
| Force rebuild | `make -B` | `just --force` | `task --force` | (always) | `must clean` or `cache = "none"` |
| Always re-run | `.PHONY` | (always) | (always unless sources set) | (always) | `phony = true` |

### Cross-compilation

| Concept | Make | Just | Taskfile | npm scripts | Mustfile |
|---|---|---|---|---|---|
| Specify target | Manual env vars | Manual env vars | Manual env vars | N/A | `--target <triple>` |
| Target groups | No | No | No | N/A | `[targets]` named groups |
| Container builds | No | No | No | N/A | `cross = "container"` |
| Toolchain discovery | No | No | No | No | `must doctor` |

---

## Migrating from Makefile

### File targets → shell recipes

Make file targets map to shell recipes with `inputs` and `outputs`:

```makefile
# Makefile
app: main.c utils.c
	gcc -o app main.c utils.c
```

```toml
# Mustfile.toml
[recipe.build]
type    = "shell"
script  = "gcc -o app main.c utils.c"
inputs  = ["main.c", "utils.c"]
outputs = ["app"]
```

Or use a first-class recipe type when available:

```toml
[recipe.build]
type    = "c-bin"
sources = ["main.c", "utils.c"]
```

### Phony targets → phony shell recipes

```makefile
# Makefile
.PHONY: lint
lint:
	cargo clippy -- -D warnings
```

```toml
# Mustfile.toml
[recipe.lint]
type   = "shell"
phony  = true
script = "cargo clippy -- -D warnings"
```

### Variables → env tables

```makefile
# Makefile
CC ?= gcc
CFLAGS := -Wall -O2

build:
	$(CC) $(CFLAGS) -o app main.c
```

```toml
# Mustfile.toml
[env]
CC     = "gcc"
CFLAGS = "-Wall -O2"

[recipe.build]
type   = "shell"
script = "$CC $CFLAGS -o app main.c"
```

### Pattern rules → manual migration

Make pattern rules have no direct equivalent. Use `must import` for a starter, then convert each pattern rule to an explicit recipe:

```makefile
# Makefile
%.o: %.c
	$(CC) -c $< -o $@
```

```toml
# Mustfile.toml — spell out each target
[recipe.main-o]
type   = "shell"
script = "$CC -c main.c -o main.o"
inputs = ["main.c"]
outputs = ["main.o"]
```

For larger C projects, prefer the `c-bin` or `c-lib` recipe types which handle compilation internally.

### Automatic import

Mustfile includes a built-in Makefile converter:

```bash
must import                              # reads ./Makefile → ./Mustfile.toml
must import --makefile GNUmakefile --out Mustfile.toml
```

This handles variable assignments, simple rules, and phony declarations. Pattern rules, `include`, and `eval` are flagged with `# TODO must:` comments in the output alongside a `MUSTFILE_IMPORT_REPORT.md`.

### Gotchas

| Make behavior | Mustfile behavior |
|---|---|
| Tabs required in recipes | TOML strings (any whitespace) |
| `$@`, `$<`, `$^` automatic variables | Use explicit names in `script` |
| `$(shell ...)` at parse time | Use a `shell` recipe that runs the command |
| Recursive make (`$(MAKE) -C dir`) | Define a separate recipe or use a shell script |
| `-include` optional dep files | No equivalent; list all deps explicitly |

---

## Migrating from Justfile

### Recipes → recipes

Justfile recipes map almost directly to Mustfile shell recipes:

```just
# Justfile
build:
    cargo build -p myapp

test build:
    cargo test -p myapp
```

```toml
# Mustfile.toml
[recipe.build]
type    = "rust-bin"
package = "myapp"

[recipe.test]
type    = "rust-test"
package = "myapp"
deps    = ["build"]
```

### Variables

```just
# Justfile
release := "false"

build:
    cargo build {{ if release == "true" { "--release" } else { "" } }}
```

```toml
# Mustfile.toml
[env.release]
CARGO_PROFILE = "release"

[recipe.build]
type    = "rust-bin"
package = "myapp"
```

Then `must build --profile release`.

### String interpolation

Just uses `{{ expression }}`. Mustfile recipes are plain shell, so use shell variable expansion:

```just
# Justfile
greet name="world":
    echo "Hello, {{name}}!"
```

```toml
# Mustfile.toml
[recipe.greet]
type   = "shell"
phony  = true
script = "echo 'Hello, world!'"
```

Mustfile does not have parameterized recipes. If you need parameterized behavior, use environment variables:

```toml
[recipe.greet]
type   = "shell"
phony  = true
script = "echo \"Hello, $NAME!\""

[recipe.greet.env]
NAME = "world"
```

### Recipe groups / modules

Just supports `mod` for importing external justfiles. Mustfile is a single `Mustfile.toml`. For large projects, split concerns by recipe type and use `deps` to wire them together.

### Gotchas

| Just behavior | Mustfile behavior |
|---|---|
| Recipes always run (no caching) | Mustfile caches by default (mtime or hash) |
| `just --list` shows recipes | `must list` |
| Shebang recipes (`#!/usr/bin/env python`) | Use a `shell` recipe that invokes the interpreter |
| `@` prefix to silence echo | Mustfile recipes are silent by default |
| `[unix]` / `[windows]` attributes | No OS filtering; use shell conditionals in `script` |
| Private recipes (`_name`) | No privacy concept; all recipes are invocable |

---

## Migrating from Taskfile

### Tasks → recipes

```yaml
# Taskfile
version: "3"
tasks:
  build:
    desc: Build the binary
    cmds:
      - go build -o bin/server ./cmd/server
    sources:
      - "**/*.go"
    generates:
      - bin/server
```

```toml
# Mustfile.toml
[recipe.build]
type    = "go-bin"
package = "./cmd/server"
```

The first-class `go-bin` recipe type handles source tracking and caching automatically.

### Task dependencies

```yaml
# Taskfile
tasks:
  generate:
    cmds: [protoc --go_out=. proto/*.proto]
  build:
    deps: [generate]
    cmds: [go build -o bin/server ./cmd/server]
```

```toml
# Mustfile.toml
[recipe.generate]
type    = "shell"
script  = "protoc --go_out=. proto/*.proto"
inputs  = ["proto/**/*.proto"]
outputs = ["**/*.pb.go"]

[recipe.build]
type    = "go-bin"
package = "./cmd/server"
deps    = ["generate"]
```

### Variables and dynamic values

```yaml
# Taskfile
tasks:
  build:
    vars:
      OUTPUT: bin/server
    cmds:
      - go build -o {{.OUTPUT}} ./cmd/server
```

```toml
# Mustfile.toml
[env]
OUTPUT = "bin/server"

[recipe.build]
type    = "go-bin"
package = "./cmd/server"
```

### Conditionals

```yaml
# Taskfile
tasks:
  lint:
    status:
      - test -f .lint-ok
    cmds:
      - golangci-lint run
      - touch .lint-ok
```

```toml
# Mustfile.toml
[recipe.lint]
type   = "shell"
phony  = true
script = "golangci-lint run"
```

Mustfile's approach is simpler: lint recipes use `phony = true` (always re-run) or `cache = "never"` rather than status checks.

### Gotchas

| Taskfile behavior | Mustfile behavior |
|---|---|
| YAML-based config | TOML-based config |
| `sources` + `generates` (mtime) | `inputs` + `outputs` (mtime or hash) |
| `deps` run before `cmds` | `deps` are resolved by the DAG scheduler |
| `dir:` changes working directory | Recipes run in `project_root` by default |
| `ignore_error:` | No equivalent (failure stops the build) |
| `defer:` cleanup | No equivalent; use shell `trap` in `script` |
| `includes:` multi-file | Single `Mustfile.toml` |
| `preconditions:` | No equivalent; use shell conditionals in `script` |
| Requires Go runtime | Single static binary |

---

## Migrating from npm scripts

The `type = "npm"` recipe type lets you call existing `package.json` scripts directly, so you don't have to rewrite anything to adopt Mustfile.

### package.json scripts → npm recipes

```json
{
  "scripts": {
    "build": "tsc --project .",
    "test": "vitest run",
    "lint": "biome check .",
    "dev": "tsc --watch"
  }
}
```

```toml
# Mustfile.toml — wraps existing package.json scripts
[project]
name = "myapp"

[recipe.build]
type   = "npm"
script = "build"

[recipe.test]
type   = "npm"
script = "test"
deps   = ["build"]

[recipe.lint]
type   = "npm"
script = "lint"

[recipe.ci]
type   = "shell"
deps   = ["lint", "test", "build"]
phony  = true
script = "echo CI passed"
```

`script` specifies which npm script to call (defaults to the recipe name). `package` specifies a subdirectory for monorepos:

```toml
[recipe.build-api]
type    = "npm"
script  = "build"
package = "packages/api"
```

### Replacing npm scripts with first-class types

Once you're ready, swap `npm` recipes for first-class types to get caching and better introspection:

```toml
# Before (delegates to package.json)
[recipe.build]
type   = "npm"
script = "build"

# After (first-class — hash caching, must explain works)
[recipe.build]
type    = "ts-bin"
package = "."
```

### Monorepo with workspaces

```json
{
  "workspaces": ["packages/api", "packages/web"]
}
```

```toml
[recipe.build-api]
type    = "ts-bin"
package = "packages/api"

[recipe.build-web]
type    = "ts-bin"
package = "packages/web"

[recipe.build]
type   = "shell"
deps   = ["build-api", "build-web"]
phony  = true
script = "echo build complete"
```

`build-api` and `build-web` execute in parallel since they have no dependency on each other.

### Gotchas

| npm scripts behavior | Mustfile behavior |
|---|---|
| `npm run <script>` always runs | `type = "npm"` always runs (Never cache). Use `ts-bin`/`ts-check`/`ts-lint` for caching |
| Pre/post hooks (`prebuild`, `postbuild`) | Not invoked. Mustfile calls `npm run <script>` directly |
| `npx` for one-off tools | Use a `shell` recipe with `npx` in the `script` |
| `npm run` lists all scripts | `must list` lists all recipes |
| Lifecycle scripts (`prepare`, `start`) | Reference by name: `script = "prepare"` |
| Requires Node.js runtime | Requires Node.js runtime (for npm). First-class `ts-*` types only need `tsc`/`biome` installed |
| `--workspace` / `-w` flags | Use `package = "packages/name"` to set working directory |

---

## Polyglot projects: the Mustfile advantage

The primary reason to choose Mustfile over the alternatives is first-class support for multiple languages in a single build file. Here is a realistic polyglot project:

### Makefile (manual wiring)

```makefile
.PHONY: build-ts lint-ts typecheck build-go test release

build-ts:
	npx tsc --project ts/app

lint-ts:
	npx biome check ts/app

typecheck: build-ts
	npx tsc --noEmit --project ts/app

build-go:
	CGO_ENABLED=0 go build -o bin/server ./cmd/server

test: typecheck build-go
	go test ./...

release: lint-ts test build-go
	tar czf release.tar.gz bin/server
```

### Mustfile (first-class types)

```toml
[project]
name = "myapp"

[recipe.build-ts]
type    = "ts-bin"
package = "ts/app"

[recipe.typecheck]
type    = "ts-check"
package = "ts/app"
deps    = ["build-ts"]

[recipe.lint-ts]
type    = "ts-lint"
package = "ts/app"

[recipe.build-go]
type    = "go-bin"
package = "./cmd/server"

[recipe.test]
type   = "shell"
deps   = ["typecheck", "build-go"]
phony  = true
script = "go test ./..."

[recipe.release]
type   = "shell"
deps   = ["lint-ts", "test", "build-go"]
phony  = true
script = "tar czf release.tar.gz bin/server"
```

Benefits:
- `ts-bin`, `ts-check`, `ts-lint`, and `go-bin` are **first-class** — Mustfile knows how to invoke `tsc` and `biome` and `go` with proper caching
- Cross-compilation for the Go binary: `must build-go --target aarch64-unknown-linux-gnu`
- `must explain build-ts` shows the cache key and whether the build would be skipped
- Parallel execution: `build-ts` and `build-go` run concurrently since they have no dependency on each other

---

## Step-by-step migration checklist

1. **Run `must import`** (if coming from Makefile) to get a starter `Mustfile.toml`
2. **Replace shell recipes with first-class types** where possible:
   - `cargo build` → `type = "rust-bin"`
   - `cargo test` → `type = "rust-test"`
   - `go build` → `type = "go-bin"`
   - `go test` → `type = "go-test"`
   - `gcc`/`clang` compiles → `type = "c-bin"` or `type = "c-lib"`
   - `tsc` compiles → `type = "ts-bin"`
   - `tsc --noEmit` → `type = "ts-check"`
    - `biome check` → `type = "ts-lint"`
    - `pip install` / `uv install` → `type = "py-bin"`
    - `pytest` → `type = "py-test"`
    - `ruff check && mypy` → `type = "py-lint"`
    - `zig build` → `type = "zig-bin"`
    - `zig build test` → `type = "zig-test"`
    - `docker build` → `type = "docker-build"`
    - `docker push` → `type = "docker-push"`
3. **Add `inputs`/`outputs`** to shell recipes for mtime caching
4. **Move variables to `[env]`** tables and use `--profile` for variants
5. **Add `[targets]`** for cross-compilation groups
6. **Run `must doctor`** to verify toolchains are available
7. **Run `must build --dry-run`** to verify the DAG looks correct
8. **Run `must list`** and `must graph`** to review the final setup

---

## Feature parity table

| Feature | Make | Just | Taskfile | Mustfile |
|---|:---:|:---:|:---:|:---:|
| Config format | Makefile DSL | Justfile DSL | YAML | TOML |
| Shell recipes | Yes | Yes | Yes | Yes |
| Rust recipes | Manual | Manual | Manual | `rust-bin` / `rust-lib` / `rust-test` |
| Go recipes | Manual | Manual | Manual | `go-bin` / `go-test` |
| C/C++ recipes | Manual | Manual | Manual | `c-bin` / `c-lib` |
| TypeScript recipes | Manual | Manual | Manual | `ts-bin` / `ts-check` / `ts-lint` |
| Python recipes | Manual | Manual | Manual | `py-bin` / `py-test` / `py-lint` |
| Zig recipes | Manual | Manual | Manual | `zig-bin` / `zig-test` |
| Docker recipes | Manual | Manual | Manual | `docker-build` / `docker-push` |
| Dependency graph | No | No | No | `must graph` |
| Cache introspection | No | No | No | `must explain` |
| Cross-compilation | Manual | Manual | Manual | `--target` + auto toolchain |
| Container builds | No | No | No | `cross = "container"` |
| Content-hash caching | No | No | No | `cache = "hash"` |
| Mtime caching | Built-in | No | `sources`/`generates` | `inputs`/`outputs` |
| Parallel execution | `make -j` | Sequential | Opt-in | Automatic (wave-based) |
| Makefile import | N/A | No | No | `must import` |
| Toolchain health check | No | No | No | `must doctor` |
| Profiles / variants | Duplicate targets | Duplicate recipes | `vars` + `includes` | `[env.<profile>]` + `--profile` |
| OS filtering | No | `[unix]`/`[windows]` | `platforms:` | Shell conditionals |
| Parameterized recipes | No | Yes | Yes (vars) | Env vars |
| Single static binary | Yes | Yes | No | Yes |
