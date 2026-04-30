# M5 Plan — Makefile Import

## Goal

Implement `must import --makefile <path> [--out <path>]` which reads a Makefile and produces a `Mustfile.toml` plus a `MUSTFILE_IMPORT_REPORT.md` summarizing what was translated, what was skipped, and what needs manual attention.

## Codebase context

- Workspace root: `crates/`
- Existing crates: `must-core`, `must-config`, `must-graph`, `must-engine`, `must-cache`, `must-recipe-shell`, `must-recipe-rust`, `must-recipe-go`, `must-recipe-cc`, `must-toolchain`, `must-cli`
- `must-cli/src/main.rs` contains the `Commands` enum and `run()` function — the `Import` subcommand must be added there
- `Cargo.toml` (workspace root) lists all `[workspace.members]` — `must-import` must be added
- `[workspace.dependencies]` in workspace `Cargo.toml` lists shared deps: `thiserror`, `serde`, `toml`, `tempfile`
- Tests follow `#[cfg(test)] mod tests` pattern inside each source file, plus fixture-based integration tests

## Design decisions

- **No `logos`/`nom`**: use a hand-written line-by-line parser — sufficient for the Makefile subset and far simpler
- **Crate name**: `must-import`, path `crates/must-import/`
- **Public API**: single `import(input: &str) -> ImportResult` function; `ImportResult` contains the TOML string and the report markdown
- **TOML output**: use `toml` crate's serialization via `serde`; produce clean, readable output
- **Error handling**: use `thiserror`; parse errors are non-fatal (malformed lines become TODO items in the report)

## Makefile syntax handled

| Input | Output |
|-------|--------|
| `VAR = value` / `:=` / `?=` | `[env.global] VAR = "value"` |
| `VAR += value` | `[env.global] VAR = "value"` (append treated as assign) |
| `.PHONY: t1 t2` | `phony = true` on matching recipes |
| `target: dep1 dep2` + tab-indented lines | `[recipe.target]` with `type="shell"`, `deps`, `script` |
| `$(shell ...)` in script lines | passed through as-is |
| `include other.mk` | TODO in report |
| `%.o: %.c` (pattern rule) | TODO in report with original snippet |
| Comments (`#`) | stripped |
| Blank lines | ignored |

## Module structure

```
crates/must-import/
├── Cargo.toml
└── src/
    ├── lib.rs          (pub fn import, pub struct ImportResult, re-exports)
    ├── lexer.rs        (line tokenization → Vec<Token>)
    ├── parser.rs       (Vec<Token> → MakefileAst)
    ├── translate.rs    (MakefileAst → MustfileOutput)
    ├── writer.rs       (MustfileOutput → TOML string)
    └── report.rs       (ImportMeta → Markdown report string)
```

## Task breakdown

---

### Task 1: Create `must-import` crate skeleton

**Files to create:**
- `crates/must-import/Cargo.toml`
- `crates/must-import/src/lib.rs`

**Files to modify:**
- `Cargo.toml` (workspace root): add `"crates/must-import"` to `[workspace.members]`

**Cargo.toml for must-import:**
```toml
[package]
name = "must-import"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
thiserror.workspace = true
serde.workspace = true
toml.workspace = true

[dev-dependencies]
tempfile.workspace = true
```

**lib.rs** must define and export:
```rust
pub mod lexer;
pub mod parser;
pub mod translate;
pub mod writer;
pub mod report;

pub use translate::ImportResult;

pub fn import(input: &str) -> ImportResult {
    let tokens = lexer::tokenize(input);
    let ast = parser::parse(tokens);
    let output = translate::translate(ast);
    output
}
```

**ImportResult** (in `translate.rs`):
```rust
pub struct ImportResult {
    pub toml: String,         // the generated Mustfile.toml content
    pub report: String,       // the MUSTFILE_IMPORT_REPORT.md content
    pub translated_count: usize,
    pub skipped_count: usize,
    pub todo_count: usize,
}
```

**Verification:** `cargo build -p must-import` succeeds (stubs can just `todo!()` for now).

---

### Task 2: Implement the lexer (`lexer.rs`)

The lexer reads the Makefile line by line and emits typed tokens. No external crate needed.

**Token types:**
```rust
pub enum Token {
    Comment(String),
    Blank,
    VarAssign { name: String, op: AssignOp, value: String },
    PhonyDecl(Vec<String>),         // .PHONY: t1 t2
    RuleHeader { target: String, deps: Vec<String> },
    RecipeLine(String),             // tab-indented line
    IncludeDirective(String),       // include path
    PatternRule { pattern: String, deps: Vec<String> }, // %.o: %.c
    Unrecognized(String),
}

pub enum AssignOp { Simple, Immediate, Conditional, Append }
```

**Tokenization rules (in order, per line):**
1. Trim trailing whitespace
2. Empty → `Blank`
3. Starts with `#` → `Comment`
4. Starts with tab → `RecipeLine` (strip leading tab)
5. Starts with `.PHONY:` → `PhonyDecl` (split remainder on whitespace)
6. Starts with `include ` → `IncludeDirective`
7. Contains `%` and matches `<pat>: <deps>` → `PatternRule`
8. Matches `<name> [:]= <value>` (VarAssign) — detect `=`, `:=`, `?=`, `+=`
9. Matches `<target>: [<deps>]` (no `%`) → `RuleHeader`
10. Otherwise → `Unrecognized`

**Important parsing details:**
- Variable name: word chars + `-` + `.` before the operator
- Rule target: no whitespace before `:`; deps are whitespace-separated after `:`
- `.PHONY` line: after `.PHONY:`, split on whitespace, filter empty

**Tests to write in `lexer.rs`:**
- `tokenize("")` → `[]`
- blank line → `[Blank]`
- comment → `[Comment]`
- `FOO = bar` → `VarAssign { name: "FOO", op: Simple, value: "bar" }`
- `FOO := bar` → `VarAssign { op: Immediate }`
- `FOO ?= bar` → `VarAssign { op: Conditional }`
- `FOO += bar` → `VarAssign { op: Append }`
- `.PHONY: all test` → `PhonyDecl(["all", "test"])`
- `all: build test` → `RuleHeader { target: "all", deps: ["build", "test"] }`
- `\techo hello` → `RecipeLine("echo hello")`
- `include foo.mk` → `IncludeDirective("foo.mk")`
- `%.o: %.c` → `PatternRule`
- multi-line Makefile produces correct token sequence

**Verification:** `cargo test -p must-import` all lexer tests pass.

---

### Task 3: Implement the parser (`parser.rs`)

The parser groups tokens into an AST. Makefile rules have: a `RuleHeader` followed by zero or more `RecipeLine` tokens.

**AST types:**
```rust
pub struct MakefileAst {
    pub nodes: Vec<AstNode>,
}

pub enum AstNode {
    Variable { name: String, op: AssignOp, value: String },
    Rule { target: String, deps: Vec<String>, recipe_lines: Vec<String>, phony: bool },
    PatternRuleTodo { original: String },
    IncludeTodo { path: String },
    Unrecognized { original: String },
}
```

**Parsing algorithm:**
1. Iterate tokens
2. `VarAssign` → emit `Variable` node
3. `RuleHeader` → start collecting; consume following `RecipeLine` tokens into `recipe_lines`; emit `Rule`
4. `PhonyDecl(names)` → record in a `HashSet<String>` of phony targets
5. `PatternRule` → emit `PatternRuleTodo` with original text
6. `IncludeDirective` → emit `IncludeTodo`
7. After pass: walk all `Rule` nodes, set `phony = true` for those whose target is in the phony set

**Tests to write in `parser.rs`:**
- Empty token list → empty AST
- Variable token → Variable node
- RuleHeader + two RecipeLines → Rule with two recipe_lines
- PhonyDecl sets phony=true on matching Rule
- PatternRule token → PatternRuleTodo
- IncludeDirective token → IncludeTodo
- Unrecognized token → Unrecognized node

**Verification:** `cargo test -p must-import` all parser tests pass.

---

### Task 4: Implement the translator and writer (`translate.rs` + `writer.rs`)

**translate.rs** converts `MakefileAst` → `ImportResult`:

```rust
pub struct MustfileOutput {
    pub env: IndexMap<String, String>,   // use std::collections::BTreeMap for sorted output
    pub recipes: Vec<OutputRecipe>,
}

pub struct OutputRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub script: String,
    pub phony: bool,
}
```

Translation rules:
- `Variable` node → add to `env` map (name → value); skip empty values
- `Rule` node → `OutputRecipe`; script = recipe_lines joined by `\n`
- `PatternRuleTodo` + `IncludeTodo` + `Unrecognized` → count as todo/skipped items, record original text for report
- Count: `translated_count` = env vars + rules; `todo_count` = pattern rules + includes; `skipped_count` = unrecognized lines

**writer.rs** serializes `MustfileOutput` to a TOML string:

Use manual string building (not serde) for readable output:
```toml
[project]
name = "imported"

[env.global]
FOO = "bar"
BAZ = "qux"

[recipe.all]
type = "shell"
deps = ["build", "test"]
phony = true
script = """
echo done
"""

[recipe.build]
type = "shell"
script = """
gcc -o app main.c
"""
```

Rules:
- Always emit `[project]\nname = "imported"\n`
- Only emit `[env.global]` section if there are env vars
- For each recipe: emit `[recipe.<name>]`, `type = "shell"`, `deps` (only if non-empty), `phony = true` (only if true), `script` as multi-line string
- Script: use TOML multiline string `"""\n<lines>\n"""` — escape any `"""` in script content

**report.rs** generates the markdown report:
```markdown
# Mustfile Import Report

## Summary
- Translated: N items
- TODO (manual review needed): N items  
- Skipped (unrecognized): N items

## Translated
- Variable `FOO`
- Rule `all` (phony)
- Rule `build`

## TODO — Needs Manual Review
- Pattern rule: `%.o: %.c` — pattern rules are not supported; convert manually
- Include: `other.mk` — include directives are not supported; inline the file

## Skipped
- Line 42: `<original>`
```

**Tests:**
- `translate()` on empty AST → empty output, all counts 0
- Variable node → appears in env map
- Rule node → appears in recipes
- PatternRuleTodo → increments todo_count, appears in report
- IncludeTodo → increments todo_count, appears in report
- Multi-recipe output → recipes in stable (insertion) order
- Writer test: `write_toml()` on a simple `MustfileOutput` → valid TOML string containing expected sections
- Report test: report markdown contains Summary section with correct counts

**Verification:** `cargo test -p must-import` all translate/writer/report tests pass.

---

### Task 5: Fixture integration tests

Create fixture files and an integration test that runs `import()` on each.

**Fixtures to create under `crates/must-import/tests/fixtures/`:**

`simple-rules.mk`:
```makefile
all: build test
	@echo done

build:
	gcc -o app main.c

test:
	./run_tests.sh
```

`vars-only.mk`:
```makefile
CC = gcc
CFLAGS = -Wall -O2
PREFIX = /usr/local
```

`phony.mk`:
```makefile
.PHONY: all clean install

all: app

clean:
	rm -f app

install: app
	cp app $(PREFIX)/bin/
```

`shell-substitution.mk`:
```makefile
GIT_HASH := $(shell git rev-parse --short HEAD)

version:
	@echo $(GIT_HASH)
```

`pattern-rules.mk`:
```makefile
%.o: %.c
	$(CC) -c $< -o $@

app: main.o util.o
	$(CC) -o $@ $^
```

`includes.mk`:
```makefile
include common.mk
include $(PLATFORM).mk

build:
	make -f subdir/Makefile
```

**Integration test file `crates/must-import/tests/integration.rs`:**
```rust
use must_import::import;

#[test]
fn fixture_simple_rules() {
    let input = include_str!("fixtures/simple-rules.mk");
    let result = import(input);
    assert!(result.toml.contains("[recipe.all]"));
    assert!(result.toml.contains("[recipe.build]"));
    assert!(result.toml.contains("[recipe.test]"));
    assert_eq!(result.todo_count, 0);
}

#[test]
fn fixture_vars_only() {
    let input = include_str!("fixtures/vars-only.mk");
    let result = import(input);
    assert!(result.toml.contains("[env.global]"));
    assert!(result.toml.contains("CC = "));
    assert_eq!(result.translated_count, 3); // 3 vars
}

#[test]
fn fixture_phony() {
    let input = include_str!("fixtures/phony.mk");
    let result = import(input);
    assert!(result.toml.contains("phony = true"));
    // all, clean, install are phony
    assert!(result.toml.contains("[recipe.clean]"));
}

#[test]
fn fixture_shell_substitution_passes_through() {
    let input = include_str!("fixtures/shell-substitution.mk");
    let result = import(input);
    // $(shell ...) should appear in script output
    assert!(result.toml.contains("$(GIT_HASH)") || result.toml.contains("GIT_HASH"));
}

#[test]
fn fixture_pattern_rules_become_todos() {
    let input = include_str!("fixtures/pattern-rules.mk");
    let result = import(input);
    assert!(result.todo_count >= 1);
    assert!(result.report.contains("Pattern rule"));
}

#[test]
fn fixture_includes_become_todos() {
    let input = include_str!("fixtures/includes.mk");
    let result = import(input);
    assert!(result.todo_count >= 2); // two include directives
    assert!(result.report.contains("Include"));
}
```

**Verification:** `cargo test -p must-import` all 6 fixture tests pass.

---

### Task 6: Wire up `must-cli::import` subcommand

**Files to modify:**
- `crates/must-cli/Cargo.toml`: add `must-import = { path = "../must-import" }` to `[dependencies]`
- `crates/must-cli/src/main.rs`: add `Import` variant to `Commands` enum + handle in `run()`

**New `Commands` variant:**
```rust
/// Import a Makefile and produce a Mustfile.toml
Import {
    /// Path to the Makefile to import (default: ./Makefile)
    #[arg(long, default_value = "Makefile")]
    makefile: PathBuf,

    /// Output path for the generated Mustfile.toml (default: ./Mustfile.toml)
    #[arg(long, default_value = "Mustfile.toml")]
    out: PathBuf,
},
```

**Handler in `run()`** (add to the `match cli.command` block, NOT async — use `std::fs`):
```rust
Commands::Import { makefile, out } => {
    let input = std::fs::read_to_string(&makefile)
        .map_err(|e| Error::Io(e))?;
    let result = must_import::import(&input);

    std::fs::write(&out, &result.toml)
        .map_err(|e| Error::Io(e))?;

    let report_path = out.with_file_name("MUSTFILE_IMPORT_REPORT.md");
    std::fs::write(&report_path, &result.report)
        .map_err(|e| Error::Io(e))?;

    println!("Imported {} → {}", makefile.display(), out.display());
    println!("  {} translated, {} TODO, {} skipped",
        result.translated_count, result.todo_count, result.skipped_count);
    println!("Report: {}", report_path.display());
}
```

**Note:** `run()` is `async fn run(cli: Cli) -> must_core::Result<()>` — the Import arm doesn't need async, just write synchronously.

**Tests to add to `must-cli/src/main.rs`** (in the existing `#[cfg(test)] mod tests` block):
```rust
#[test]
fn test_import_roundtrip_via_must_import() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mk = tmp.path().join("Makefile");
    std::fs::write(&mk, "build:\n\tgcc -o app main.c\n").unwrap();
    let out = tmp.path().join("Mustfile.toml");
    // call import directly (not via CLI) to keep test simple
    let input = std::fs::read_to_string(&mk).unwrap();
    let result = must_import::import(&input);
    std::fs::write(&out, &result.toml).unwrap();
    assert!(out.exists());
    assert!(result.toml.contains("[recipe.build]"));
}
```

**Verification:** `cargo test --workspace` all tests pass; `cargo build -p must-cli` succeeds; `cargo run -p must-cli -- import --help` prints usage.

---

## Commit order

1. Commit Task 1 (skeleton): `"feat(must-import): add crate skeleton"`
2. Commit Task 2 (lexer): `"feat(must-import): implement line tokenizer"`
3. Commit Task 3 (parser): `"feat(must-import): implement AST parser"`
4. Commit Task 4 (translate + writer + report): `"feat(must-import): implement translator, TOML writer, and report generator"`
5. Commit Task 5 (fixtures): `"test(must-import): add fixture integration tests"`
6. Commit Task 6 (CLI): `"feat(must-cli): add import subcommand"`
