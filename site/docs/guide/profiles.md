# Profiles & Environments

## Environment variable layers

Environment variables are resolved in layers. Later layers override earlier ones:

```
1. System environment
2. [env] section in Mustfile.toml
3. [env.<profile>] section
4. Recipe-level env field
5. MUST_ARGS environment variable
```

## Global environment

```toml
[env]
LOG_LEVEL = "info"
OPT_LEVEL = "2"
REGISTRY  = "ghcr.io/myorg"
```

## Profile overrides

```toml
[env]
OPT_LEVEL = "2"

[env.release]
OPT_LEVEL = "3"
LOG_LEVEL = "warn"
```

Select a profile:

```bash
must build --profile release
```

## Recipe-level environment

```toml
[recipe.build]
type    = "rust-bin"
package = "my-app"

[recipe.build.env]
RUSTFLAGS = "-C target-cpu=native"
```

## Variable interpolation

Use `${VAR}` syntax for runtime expansion:

```toml
[env]
TARGET_DIR = "target/${TARGET}"
IMAGE_TAG  = "${REGISTRY}/api:${VERSION}"
```

`$VAR` and `${VAR}` are both supported. Undefined variables expand to empty strings.

## MUST_ARGS

Pass extra flags from CI or wrappers:

```bash
MUST_ARGS="--release --fail-fast" must build
```

## Shell environment inheritance

Recipes inherit the full system environment plus all must-configured variables. This means:

- `PATH` is available by default
- Any env var set in CI (GitHub Actions, etc.) is accessible
- must env vars are layered on top, not replacements
