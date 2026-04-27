# Mustfile — Tasks

Milestone-by-milestone task list for v1. Check off as tasks complete. Each milestone ends in a runnable, end-to-end demonstrable artifact.

Legend: `[ ]` = todo · `[~]` = in progress · `[x]` = done · `[-]` = deferred / dropped

---

## M0 — Project bootstrap

- [ ] `cargo new --vcs git mustfile` (already in `/Users/greg/git/mustfile`; init in place)
- [ ] Convert root to a Cargo workspace (`Cargo.toml` with `[workspace]`)
- [ ] Add CI scaffold: `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace`
- [ ] `rust-toolchain.toml` pinning a stable Rust version
- [ ] `.editorconfig`, `.gitignore`, `LICENSE` (MIT or Apache-2.0 — pick one)
- [ ] `README.md` linking to `docs/OVERVIEW.md`

**Done when:** `cargo build --workspace` succeeds on an empty workspace and CI is green.

---

## M1 — Skeleton + shell recipes (week 1–2)

Goal: replace simple Makefiles. End-state: `must build` runs a shell recipe from a `Mustfile.toml` with mtime caching and `-j` parallelism.

### Crates to create

- [ ] `must-core` — traits (`Recipe`, `Toolchain`, `Cache`), `Error` enum, `BuildContext`, `CacheKey`, `CacheLookup`
- [ ] `must-config` — serde model for `Mustfile.toml`, validation
- [ ] `must-graph` — DAG, topo sort, cycle detection, wave grouping
- [ ] `must-engine` — Tokio scheduler, env composition, recipe dispatch
- [ ] `must-cache` — on-disk store under `.mustfile/cache/`, mtime strategy
- [ ] `must-recipe-shell` — generic shell with mtime + opt-in hash
- [ ] `must-cli` — clap entry point, `tracing` subscriber

### Concrete tasks

- [ ] `must-core`: define `Recipe`, `Toolchain`, `Cache` traits + `Error` enum
- [ ] `must-config::schema`: `Project`, `Env`, `Targets`, `Recipe` (untagged enum on `type`), profile merge
- [ ] `must-config::validate`: name uniqueness, dep references resolve, no obvious cycles (full check is in `must-graph`)
- [ ] `must-graph::dag`: Kahn's algorithm topo sort, cycle reporting with full path
- [ ] `must-engine::env`: layered env composition (process → global → profile → recipe → toolchain)
- [ ] `must-engine::scheduler`: wave executor, `-j` semaphore, `--fail-fast`
- [ ] `must-cache::store`: directory layout `.mustfile/cache/<sha-prefix>/<sha-rest>/`, sled index
- [ ] `must-cache::mtime`: max-input vs. min-output comparison
- [ ] `must-recipe-shell`: spawn shell (`sh -c` on unix, PowerShell on windows — start unix-only), capture output, exit code
- [ ] `must-cli`: clap arg parser, top-level subcommands `build`, `test`, `run`, `<recipe>`, `list`, `clean`, `--dry-run`, `-j`
- [ ] Integration test scenario: `simple-shell` (one recipe, one input, one output, mtime cache hit/miss)
- [ ] Integration test scenario: `deps-dag` (three recipes with deps; verify topo order)
- [ ] Integration test scenario: `parallelism` (recipes with sleeps + `-j 4` faster than `-j 1`)

**Done when:** `must build` works against a real `Mustfile.toml` with shell recipes, mtime caching, and parallelism. `cargo test --workspace` is green.

---

## M2 — Hash caching + Rust module (week 3–4)

Goal: dogfood — Mustfile builds Mustfile.

### Concrete tasks

- [ ] `must-cache::hash`: SHA-256 over recipe type + sorted inputs (file content) + env + flags + toolchain id
- [ ] `must-cache`: support `cache = "hash"` for shell recipes; default-hash for first-class recipes
- [ ] `must-recipe-rust`:
  - [ ] `rust-bin` (release/debug, features, target dir, profile)
  - [ ] `rust-lib` (cdylib/staticlib variants)
  - [ ] `rust-test` (cargo test invocation, test filter passthrough)
- [ ] `must-cli::explain`: `--explain RECIPE` prints cache-key inputs and which one changed
- [ ] Dogfood: write `mustfile/Mustfile.toml` defining `build`, `test`, `release` for the workspace
- [ ] CI: build with `cargo run -p must-cli -- build` to validate the bootstrap path
- [ ] Integration test scenarios: `rust-bin`, `shell-with-hash-cache`

**Done when:** `must build` builds the `must` binary itself. Branch-switch test demonstrates no spurious rebuilds.

---

## M3 — Cross-compile + Go module (week 5–6)

Goal: `must build --target aarch64-linux-gnu` works for Rust and Go.

### Concrete tasks

- [ ] `must-toolchain::triple`: parse + validate target triples, reject unknown
- [ ] `must-toolchain::discover` (Rust): `rustup target list --installed`
- [ ] `must-toolchain::discover` (Go): `GOOS`/`GOARCH` mapping, no probe
- [ ] `must-toolchain::LocalToolchain`: produce env (`CC`, `CARGO_TARGET_<TRIPLE>_LINKER`, `GOOS`, `GOARCH`, etc.)
- [ ] `must-toolchain`: actionable "missing toolchain" errors with install hint per OS
- [ ] `must-recipe-go`:
  - [ ] `go-bin` (package path, ldflags, build tags, GOOS/GOARCH from triple)
  - [ ] `go-test`
- [ ] `must-cli::target`: `--target` flag (multi-allowed), `[targets]` group expansion
- [ ] Integration test scenarios: `rust-bin-cross-aarch64`, `go-bin-cross`, `multi-target-release`

**Done when:** `must release --profile release` cross-compiles a Rust binary and a Go binary for at least two non-host triples.

---

## M4 — C/C++ module + container toolchain (week 7–8)

Goal: cross-compile C/C++ via container.

### Concrete tasks

- [ ] `must-toolchain::discover` (C/C++): scan PATH for `<triple>-gcc` / `<triple>-clang`, common locations (`/usr/bin`, Homebrew prefixes, NDK paths)
- [ ] `must-toolchain::LocalToolchain` (C/C++): set `CC`, `AR`, `LD`, `CFLAGS`, sysroot
- [ ] `must-toolchain::ContainerToolchain`: docker/podman exec wrapper, project-root bind mount, image selection
- [ ] `must-recipe-cc`:
  - [ ] `c-bin` (sources, includes, link libs, flags)
  - [ ] `c-lib` (static and shared)
- [ ] Container image registry: map `<triple> + recipe-type → image` (initial: cross-rs for Rust, dockcross/* for C/C++)
- [ ] Per-recipe override: `[recipe.<name>.cross]` with `linker`, `cross = "container"`
- [ ] Integration test scenarios: `cc-bin-local`, `cc-bin-container` (gated on docker available in CI)

**Done when:** A C "hello world" cross-compiles to aarch64 via both local toolchain (when present) and container.

---

## M5 — Makefile import (week 9–10)

Goal: `must import` produces a runnable starter `Mustfile.toml` from a real-world Makefile.

### Concrete tasks

- [ ] `must-import::lexer`: `logos`-based tokenizer for variables, rules, recipes, conditionals, includes
- [ ] `must-import::parser`: `nom`-based AST builder
- [ ] `must-import::translate`:
  - [ ] Variable assignments → `[env]`
  - [ ] Simple rules → `[recipe.<name>]` with `type = "shell"`, `deps`, `script`
  - [ ] Phony targets → `phony = true`
  - [ ] `$(shell ...)` → inline `$(...)` shell substitution
  - [ ] Pattern rules → preserve as TODO comments with original snippet
  - [ ] Includes → flagged as TODO; include path resolution out of scope
- [ ] `must-import::report`: emit `MUSTFILE_IMPORT_REPORT.md` listing translated, skipped, and TODO items
- [ ] `must-cli::import`: subcommand wiring + `--makefile PATH` and `--out PATH` flags
- [ ] Fixture corpus: `simple-rules`, `vars`, `phony`, `shell-substitution`, `includes-flagged`, `pattern-rules-flagged`
- [ ] Manual UX pass: import 3+ real OSS Makefiles (e.g. one C project, one mixed-lang, one esoteric); ensure errors are useful

**Done when:** `must import --makefile <real-makefile>` produces a Mustfile.toml that runs at least the basic targets (`make all` → `must all`).

---

## M6 — Polish (week 11–12)

Goal: tag v1.

### Concrete tasks

- [ ] `must doctor`: check toolchains, container runtime, cache health (orphans, size); print actionable hints
- [ ] `must graph`: text and DOT output (`--format text|dot|mermaid`)
- [ ] Error message audit: every `Error` variant has actionable context
- [ ] `--explain` polish: include changed-input diff, toolchain id, env diff
- [ ] Cross-platform paths: ensure shell recipes work on macOS + Linux (Windows shell deferred)
- [ ] Release tooling:
  - [ ] GitHub Actions matrix builds for x86_64-linux-gnu, aarch64-linux-gnu, x86_64-apple-darwin, aarch64-apple-darwin (Windows deferred)
  - [ ] `cargo install --locked mustfile` works from crates.io
  - [ ] Prebuilt binaries attached to GitHub releases
  - [ ] `install.sh` script (Homebrew formula deferred)
- [ ] Documentation: `docs/USER_GUIDE.md`, schema reference, recipe-type reference
- [ ] CHANGELOG.md following Keep a Changelog
- [ ] Tag `v1.0.0`

**Done when:** `cargo install --locked mustfile` installs from crates.io; the resulting binary runs the integration corpus successfully.

---

## Backlog (post-v1)

- [ ] Runtime Makefile mode: `must -f Makefile`
- [ ] Toolchain auto-provisioning (Rust via rustup; C/C++ via curated tarballs)
- [ ] Remote cache (S3 / HTTP backend)
- [ ] JS/TS recipe module (`node-bin`, `bundle-vite`, `bundle-esbuild`)
- [ ] Python recipe module (`py-wheel`)
- [ ] Plugin system (dynamic-linked or wasm-based recipe types)
- [ ] Windows shell support (PowerShell or busybox)
- [ ] Watch mode (`must watch <recipe>`)
- [ ] Editor integrations (VS Code, IntelliJ)
- [ ] `must init` interactive scaffolding

---

## Cross-cutting tracks (run alongside milestones)

- [ ] **Testing.** Integration test corpus grows with each milestone. Aim for one real-world scenario per recipe type.
- [ ] **Documentation.** Update `OVERVIEW.md` and `DESIGN.md` whenever a design decision changes. Note design changes in `SCRATCHPAD.md` first.
- [ ] **Performance.** Track `must build` cold/warm time on a reference project; flag regressions.
- [ ] **Telemetry / opt-in metrics.** Decide before v1 whether to ship any (probably no).
