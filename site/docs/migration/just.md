# Migrating from Just

just is a command runner with a Make-like syntax. must provides a superset of just's functionality with language-aware recipes.

## Concept mapping

| just | must |
|------|------|
| `justfile` | `Mustfile.toml` |
| `recipe_name:` | `[recipe.recipe_name]` |
| Recipe body (indented) | `script = "..."` |
| `recipe_name: dep1 dep2` | `deps = ["dep1", "dep2"]` |
| `export VAR := "value"` | `[env] VAR = "value"` |
| `{{ VAR }}` | `${VAR}` |
| `[private]` | No direct equivalent — omit from Mustfile |
| `[unix]` / `[windows]` | Use shell conditionals or separate recipes |
| `just --list` | `must list` |
| `just --dry-run` | `must --dry-run` |
| `just --watch` | `must watch` |
| `just recipe_name` | `must build recipe_name` |

## Side-by-side

**justfile:**

```just
export OPT_LEVEL := "2"

build:
    cargo build --release

test: build
    cargo test

lint:
    cargo clippy -- -D warnings

clean:
    cargo clean
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

[recipe.clean]
type   = "shell"
phony  = true
script = "cargo clean"
```

## Key differences

| Feature | just | must |
|---------|------|------|
| Language awareness | No — all recipes are shell | Yes — 41 first-class recipe types |
| Caching | No built-in caching | Hash/mtime/never per recipe |
| Cross-compilation | Manual | Automatic with target groups |
| Dependency graph | Flat | DAG with parallel waves |
| Plugins | No | Lua plugins with stdlib |
| Build logs | No | Per-recipe log storage |
| Outdated checking | No | `must outdated` |

## Migration steps

1. Rename `justfile` to `Mustfile.toml`
2. Wrap each recipe in `[recipe.<name>]` with `type = "shell"`
3. Move exports to `[env]` section
4. Convert `{{ VAR }}` to `${VAR}`
5. Upgrade shell recipes to first-class types where possible
