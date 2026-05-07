# Importing from Makefile

must can import existing Makefiles and generate a starter `Mustfile.toml`.

## Basic usage

```bash
must import Makefile
```

This reads your Makefile and generates:

1. `Mustfile.toml` — converted recipes
2. `import-report.md` — detailed report of what was converted and what needs manual review

## What gets imported

| Makefile construct | Mustfile equivalent |
|-------------------|---------------------|
| `target: deps` | `[recipe.target]` with `deps` |
| `$(VAR)` | `${VAR}` |
| Recipe body | `script = "..."` with `type = "shell"` |
| `.PHONY` | `phony = true` |
| Pattern rules | Skipped (needs manual conversion) |
| Shell functions | Preserved as-is in script |
| Automatic variables (`$@`, `$<`, `$^`) | Expanded inline |

## Import report

The generated `import-report.md` contains:

- Successfully converted recipes
- Skipped rules (pattern rules, implicit rules)
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
