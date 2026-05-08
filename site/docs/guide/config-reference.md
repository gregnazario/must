# Mustfile.toml Reference

The `Mustfile.toml` is the single source of truth for your project's build configuration.

## Top-level sections

```toml
[project]          # required — project metadata
[env]              # optional — global environment variables
[env.<profile>]    # optional — profile-specific overrides
[targets]          # optional — cross-compilation target groups
[recipe.<name>]    # required — one or more recipes
[include]          # optional — include other Mustfile.toml files
```

## `[project]`

```toml
[project]
name    = "my-app"       # required
version = "1.0.0"        # optional
```

## `[env]`

Environment variables available to all recipes. Supports variable interpolation.

```toml
[env]
OPT_LEVEL  = "2"
TARGET_DIR = "target/${TARGET}"    # ${VAR} expanded at runtime
REGISTRY   = "ghcr.io/myorg"

[env.release]
OPT_LEVEL = "3"
```

Resolution order (later wins):

1. System environment
2. `[env]` section
3. `[env.<profile>]` section
4. Recipe-level `env` field
5. `MUST_ARGS` environment variable

## `[targets]`

Define named groups of compilation targets:

```toml
[targets]
default = ["x86_64-unknown-linux-gnu"]
release = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
]
```

Use with `--target`:

```bash
must build --target release
must build --target aarch64-unknown-linux-gnu
```

## `[recipe.<name>]`

Every recipe has these fields:

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | **Required.** One of the 41 recipe type identifiers |
| `package` | string | Package/target/module path (most types) |
| `script` | string | Shell command (`shell` / `npm` types) |
| `script_win` | string | Windows override for `script` (falls back to `script` if unset) |
| `deps` | string[] | Dependencies — must complete before this recipe |
| `env` | map | Recipe-specific environment variables |
| `inputs` | string[] | Input file globs (for cache hashing) |
| `outputs` | string[] | Output file paths |
| `phony` | bool | Always re-run, skip caching (default: false) |
| `cache` | string | `"hash"`, `"mtime"`, or `"never"` |
| `workdir` | string | Working directory relative to project root |
| `features` | string[] | Feature flags (Rust recipes) |
| `image` | string | Docker image name (Docker recipes) |
| `dockerfile` | string | Path to Dockerfile |
| `build_args` | map | Docker build args |
| `plugin` | string | Plugin name (plugin type) |
| `sources` | string[] | Source files (C recipes) |
| `includes` | string[] | Include directories (C recipes) |
| `link_libs` | string[] | Libraries to link (C recipes) |
| `ldflags` | string | Linker flags (Go recipes) |

### Cross-compilation overrides

```toml
[recipe.cli.cross]
"x86_64-unknown-linux-gnu" = {}
"aarch64-unknown-linux-gnu" = { linker = "aarch64-linux-gnu-gcc" }
```

### Examples

**Rust binary:**

```toml
[recipe.build]
type     = "rust-bin"
package  = "my-app"
features = ["cli", "default"]
```

**Go service:**

```toml
[recipe.api]
type    = "go-bin"
package = "./cmd/api"
ldflags = "-s -w -X main.version=$(git describe --tags --always)"
```

**TypeScript with deps:**

```toml
[recipe.build]
type    = "ts-bin"
package = "tsconfig.json"

[recipe.check]
type    = "ts-check"
package = "tsconfig.json"
deps    = ["build"]
```

**Python:**

```toml
[recipe.test]
type    = "py-test"
package = "tests/"

[recipe.lint]
type    = "py-lint"
package = "src/"
```

**Docker:**

```toml
[recipe.image]
type       = "docker-build"
image      = "${REGISTRY}/api:latest"
dockerfile = "Dockerfile"
build_args = { VERSION = "1.0.0" }
deps       = ["build"]
```

**Shell with caching:**

```toml
[recipe.proto]
type    = "shell"
inputs  = ["proto/**/*.proto"]
outputs = ["src/proto/mod.rs"]
cache   = "hash"
script  = "protoc --rust_out=src/proto proto/*.proto"
```

**Lua plugin:**

```toml
[recipe.codegen]
type   = "plugin"
plugin = "protoc"
```

## `[include]`

Include another Mustfile.toml (relative path):

```toml
[include]
paths = ["libs/core/Mustfile.toml"]
```

## Layered environments

Environment variables are resolved in layers, with later layers overriding earlier ones:

```
system env → [env] → [env.<profile>] → recipe.env → MUST_ARGS
```

Use `MUST_ARGS` to pass extra args from CI:

```bash
MUST_ARGS="--release" must build
```
