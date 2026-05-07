# Migrating from Taskfile

Taskfile is a YAML-based task runner. must uses TOML and provides language-aware recipes.

## Concept mapping

| Taskfile | must |
|----------|------|
| `Taskfile.yml` | `Mustfile.toml` |
| `tasks:` | `[recipe.<name>]` sections |
| `cmds:` | `script = "..."` |
| `deps:` | `deps = [...]` |
| `env:` | `[env]` or recipe-level `env` |
| `sources:` | `inputs` |
| `generates:` | `outputs` |
| `status:` | Automatic via cache strategy |
| `dir:` | `workdir` |
| `task --list` | `must list` |
| `task --dry` | `must --dry-run` |
| `task --watch` | `must watch` |

## Side-by-side

**Taskfile.yml:**

```yaml
version: '3'

env:
  OPT_LEVEL: "2"

tasks:
  build:
    cmds:
      - cargo build --release -p my-app
    sources:
      - src/**/*.rs
    generates:
      - target/release/my-app

  test:
    deps: [build]
    cmds:
      - cargo test -p my-app

  lint:
    cmds:
      - cargo clippy -- -D warnings
```

**Mustfile.toml:**

```toml
[project]
name = "my-app"

[env]
OPT_LEVEL = "2"

[recipe.build]
type    = "rust-bin"
package = "my-app"

[recipe.test]
type    = "rust-test"
package = "my-app"
deps    = ["build"]

[recipe.lint]
type   = "shell"
phony  = true
script = "cargo clippy -- -D warnings"
```

## Key differences

| Feature | Taskfile | must |
|---------|----------|------|
| Config format | YAML | TOML |
| Language awareness | No | 41 first-class recipe types |
| Caching | `sources`/`generates` with `status` check | Automatic hash/mtime/never |
| Cross-compilation | Manual env vars | Target groups + auto toolchain |
| Parallel execution | Yes | Yes (DAG waves) |
| Plugins | No | Lua plugins |

## Migration steps

1. Convert YAML to TOML syntax
2. Wrap each task in `[recipe.<name>]`
3. Move `cmds` to `script` (join multi-cmd with `&&`)
4. Move `deps` directly
5. Replace `sources`/`generates` with appropriate cache strategy
6. Upgrade to first-class recipe types where possible
