# Importing Build Files

must can import existing build files and produce a starter `Mustfile.toml`. Supports Makefiles, Justfiles, and Taskfiles.

## Supported formats

| Format | File | `--format` flag | Auto-detected by filename |
|--------|------|-----------------|---------------------------|
| Make | `Makefile` | `make` (default) | Any file not matching below |
| Just | `justfile`, `Justfile` | `just` | `justfile` or `justfile.*` |
| Taskfile | `Taskfile.yml`, `Taskfile.yaml` | `taskfile` | `Taskfile*.yml` / `Taskfile*.yaml` |

## Basic usage

```bash
# Auto-detect format from filename
must import --makefile Makefile
must import --makefile justfile
must import --makefile Taskfile.yml

# Explicit format
must import --makefile build.tasks --format just
```

This generates:

1. `Mustfile.toml` — converted recipes
2. `MUSTFILE_IMPORT_REPORT.md` — detailed report

## What gets imported

### From Makefile

| Makefile construct | Mustfile equivalent |
|-------------------|---------------------|
| `target: deps` | `[recipe.target]` with `deps` |
| `$(VAR)` | `${VAR}` |
| Recipe body | `script = "..."` with `type = "shell"` |
| `.PHONY` | `phony = true` |
| Pattern rules | Skipped (needs manual conversion) |
| Shell functions | Preserved as-is in script |
| Automatic variables (`$@`, `$<`, `$^`) | Expanded inline |

### From Justfile

| Justfile construct | Mustfile equivalent |
|--------------------|---------------------|
| `recipe_name:` | `[recipe.recipe_name]` |
| `recipe_name: dep1 dep2` | `deps = ["dep1", "dep2"]` |
| Recipe body | `script = "..."` with `type = "shell"` |
| `export VAR = "value"` | `[env] VAR = "value"` |
| `set shell := [...]` | Skipped (noted in report) |
| `[linux]` / `[windows]` attributes | Skipped (noted in report) |

### From Taskfile

| Taskfile construct | Mustfile equivalent |
|--------------------|---------------------|
| `tasks: name: cmds:` | `[recipe.name]` with script |
| `deps: [build]` | `deps = ["build"]` |
| `desc: "description"` | Noted in report |
| `cmds:` list | Joined into single script |

## Import report

The generated `MUSTFILE_IMPORT_REPORT.md` contains:

- Successfully converted recipes
- Skipped rules (pattern rules, attributes, etc.)
- Variables that need manual review
- Suggestions for converting to first-class recipe types

Example:

```markdown
# Import Report

## Converted (12 recipes)
- build → shell
- test → shell
- clean → shell (phony)
- lint → shell (phony)

## Needs review (3 items)
- `release` uses pattern rule — convert manually
- `deploy` calls SSH — may need secrets handling
- `CFLAGS` has complex expansion — verify

## Suggestions
- `build` compiles C code → consider `c-bin` type
- `test` runs pytest → consider `py-test` type
```

## Converting to first-class types

After import, consider upgrading shell recipes to first-class types:

```toml
# Before (imported)
[recipe.build]
type   = "shell"
script = "cargo build --release -p my-app"

# After (first-class)
[recipe.build]
type    = "rust-bin"
package = "my-app"
```

Benefits: automatic caching, cross-compilation support, toolchain version tracking.

## Or: use bridge mode instead

If you'd rather not convert, just use `must` directly against your existing build files — no import needed:

```bash
cd my-makefile-project
must build    # auto-detects Makefile, runs: make
```

See [Bridge Recipes](../recipes/bridge.md) for details.
