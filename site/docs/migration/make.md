# Migrating from Make

A practical guide to converting Makefiles to Mustfile.toml.

## Concept mapping

| Make | must |
|------|------|
| `Makefile` | `Mustfile.toml` |
| `target: deps` | `[recipe.target]` with `deps` |
| Recipe body (tab-indented) | `script = "..."` |
| `$(VAR)` / `${VAR}` | `${VAR}` (same syntax) |
| `.PHONY: clean` | `phony = true` |
| `$(MAKE) -C subdir` | `workdir = "subdir/"` |
| `ifeq` / `ifdef` | `[env.<profile>]` or recipe-level `env` |
| `include other.mk` | `[include] paths = [...]` |
| `@echo "message"` | `script = "echo 'message'"` |
| Pattern rules (`%.o: %.c`) | No equivalent — use a loop or first-class type |
| `$(shell ...)` | Use must's built-in variable expansion |
| `$@`, `$<`, `$^` | Expanded inline during import |

## Step-by-step migration

### 1. Import

```bash
must import Makefile
```

This generates `Mustfile.toml` and `import-report.md`.

### 2. Review the import report

Check for:

- Pattern rules that need manual conversion
- Complex variable expansions
- Implicit dependencies

### 3. Convert to first-class types

Replace generic shell recipes with language-specific types:

```toml
# Before
[recipe.build]
type   = "shell"
script = "cargo build --release -p my-app"

# After
[recipe.build]
type    = "rust-bin"
package = "my-app"
```

### 4. Move variables to [env]

```makefile
# Makefile
CFLAGS := -O2 -Wall
TARGET := my-app
```

```toml
# Mustfile.toml
[env]
CFLAGS = "-O2 -Wall"
TARGET = "my-app"
```

### 5. Convert phony targets

```makefile
# Makefile
.PHONY: clean test
clean:
    rm -rf build/
test:
    cargo test
```

```toml
# Mustfile.toml
[recipe.clean]
type   = "shell"
phony  = true
script = "rm -rf build/"

[recipe.test]
type    = "rust-test"
package = "my-app"
```

### 6. Remove tab indentation

must uses TOML strings — no tab-significant whitespace.

## Side-by-side comparison

**Makefile:**

```makefile
CFLAGS := -O2 -Wall
SRC    := $(wildcard src/*.c)
OBJ    := $(SRC:.c=.o)

my-app: $(OBJ)
    $(CC) $(CFLAGS) -o $@ $^

%.o: %.c
    $(CC) $(CFLAGS) -c $< -o $@

.PHONY: clean test
test: my-app
    ./my-app --test

clean:
    rm -f $(OBJ) my-app
```

**Mustfile.toml:**

```toml
[project]
name = "my-app"

[env]
CFLAGS = "-O2 -Wall"

[recipe.build]
type    = "c-bin"
sources = ["src/*.c"]
includes = ["src/"]

[recipe.test]
type   = "shell"
deps   = ["build"]
phony  = true
script = "./my-app --test"

[recipe.clean]
type   = "shell"
phony  = true
script = "rm -f src/*.o my-app"
```
