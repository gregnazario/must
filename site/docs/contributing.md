# Contributing

Contributions are welcome. This guide covers building, testing, and submitting changes.

## Setup

**Requirements:**
- Rust stable (see `rust-toolchain.toml`)
- `cargo`, `rustfmt`, `clippy` (installed via rustup)

**Build and test:**

```bash
must build          # cargo build
must test           # cargo test
must lint           # cargo clippy -- -D warnings
must fmt            # cargo fmt --check
must ci             # runs fmt + lint + test
```

Or use `must` to build itself — see the root `Mustfile.toml`.

## Project structure

```
crates/
├── must-core/           # Core types, traits, error types, command utils
├── must-config/         # Mustfile.toml parsing and validation
├── must-graph/          # Dependency DAG, topological sort, waves
├── must-cache/          # DiskCache (sled), hash/mtime strategies
├── must-toolchain/      # Toolchain discovery (go, rustc, etc.)
├── must-engine/         # Build engine, scheduler, env composition
├── must-plugin/         # Lua plugin runtime (mlua)
├── must-import/         # Makefile → Mustfile.toml converter
├── must-cli/            # CLI binary (clap, all recipe wiring)
├── must-recipe-shell/   # Shell recipe type
├── must-recipe-rust/    # Rust recipe types (bin, lib, test)
├── must-recipe-go/      # Go recipe types
├── must-recipe-cc/      # C/C++ recipe types
├── must-recipe-ts/      # TypeScript recipe types
├── must-recipe-py/      # Python recipe types
├── must-recipe-zig/     # Zig recipe types
├── must-recipe-docker/  # Docker recipe types
├── must-recipe-java/    # Java recipe types
├── must-recipe-kotlin/  # Kotlin recipe types
├── must-recipe-swift/   # Swift recipe types
├── must-recipe-dotnet/  # .NET recipe types
├── must-recipe-ruby/    # Ruby recipe types
├── must-recipe-dart/    # Dart recipe types
├── must-recipe-elixir/  # Elixir recipe types
├── must-recipe-flutter/ # Flutter recipe types
├── must-recipe-nim/     # Nim recipe types
└── must-recipe-precompiled/  # Precompiled binary downloads
```

## Adding a new recipe type

1. **Create the crate:** `crates/must-recipe-<lang>/`
2. **Implement the `Recipe` trait** from `must-core`:

```rust
use must_core::{BuildContext, CacheKey, Error, Recipe, RecipeOutput, Result};

pub struct MyBinRecipe {
    pub name: String,
    pub package: String,
    pub deps: Vec<String>,
    pub env: HashMap<String, String>,
}

impl Recipe for MyBinRecipe {
    fn name(&self) -> &str { &self.name }
    fn recipe_deps(&self) -> &[String] { &self.deps }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        let mut cmd = Command::new("my-tool");
        cmd.arg("build").arg(&self.package);
        cmd.current_dir(&ctx.project_root);
        // ... run command, return output
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> { ... }
    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> { ... }
    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> { ... }
}
```

3. **Add the variant** to `RecipeType` in `must-config/src/schema.rs`
4. **Wire it up** in `must-cli/src/main.rs` (match arm in the recipe builder)
5. **Add tests** — unit tests in the recipe crate, integration test in CLI
6. **Add docs** — recipe page in `site/docs/recipes/`, entry in index
7. **Add an example** in `examples/`

## Testing

```bash
cargo test --workspace           # run all tests
cargo test -p must-recipe-rust   # test one crate
cargo clippy --workspace -- -D warnings  # lint
cargo fmt --check                # format check
```

**Writing tests:**
- Unit tests go in the same file under `#[cfg(test)] mod tests`
- Real-execution tests should use `match` on `Ok`/`ToolNotFound`/`RecipeFailed` to handle missing toolchains
- Cache tests use `match` pattern (not `.unwrap()`) to avoid flakes
- Use `tempfile::TempDir` for filesystem tests

## Documentation

- **Rust docs:** `///` on all public items, `//!` on crate root
- **Doc site:** MkDocs Material in `site/` — preview with `mkdocs serve`
- **Examples:** `examples/` directory with realistic Mustfile.toml files

## Pull request checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] New public items have `///` doc comments
- [ ] New recipe types have a doc page and example

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
