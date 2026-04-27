# Mustfile — Scratchpad

Working notes: open questions, alternatives we considered and rejected, future ideas, and unresolved details. This file is intentionally messier than the others — capture first, organize later.

---

## Open questions (need decisions before or during v1)

### Workspace / multi-project layout
- Single `Mustfile.toml` at the project root, or support nested workspaces (one Mustfile per crate-like unit)?
- **Tentative:** single root file in v1. Add a `[workspace]` section + per-member files later if real demand surfaces. Cargo's pattern is a good model.

### Phony recipes
- Make has `.PHONY`. Mustfile recipes that produce no `outputs` are implicitly always-run. Should we still surface `phony = true` as an explicit toggle for clarity?
- **Tentative:** yes. `phony = true` skips cache lookup entirely; useful for recipes like `clean`, `serve`, `watch`.

### Variable interpolation in TOML
- TOML doesn't natively support variable substitution. Should we add `${VAR}` expansion inside string values (env, paths, scripts)?
- **Tentative:** yes — but only for env vars and `[project]` fields, expanded at parse time. Avoids the rabbit hole of GNU Make's expansion semantics.

### Reserved recipe names
- Should `build`, `test`, `release`, `run`, `check`, `clean` be reserved? Or just conventional?
- **Tentative:** conventional, not reserved. `must <recipe>` works for any name; `must build` just defaults to a recipe named `build` if present.

### What does `must run` mean?
- Option A: `must run RECIPE` is identical to `must RECIPE` (just an explicit form).
- Option B: `must run RECIPE` runs the *output* of a recipe (e.g. for `rust-bin` types, executes the produced binary).
- **Tentative:** B. It distinguishes "build then execute" from "trigger this recipe", which matches how users think about `cargo run` vs. `cargo build`. `must <recipe>` covers the latter.

### Cache eviction
- Unbounded caches eventually fill disks. LRU? Size-bound? Time-bound?
- **Tentative:** v1 ships with no auto-eviction; `must clean --cache` is the only knob. Add a configurable size cap in v0.2.

### Telemetry
- Almost every modern build tool adds opt-in telemetry. Worth it for v1?
- **Tentative:** no. Adds a CI/legal/UX cost without paying off until there's a user base.

### Lock file?
- Bazel/Buck have lock files for hermeticity. Mustfile-the-task-runner probably doesn't need one. Mustfile-the-build-system might.
- **Tentative:** no lock file in v1. Re-evaluate when the language modules push toward determinism.

---

## Alternatives considered (and rejected, for now)

### Pure DSL (rejected during brainstorming)
A custom Mustfile syntax (Make-like or Just-like) is more ergonomic for the domain but carries a parser/IDE/learning-curve cost. TOML + shell is "good enough" and ships faster.

### Pure TOML (no shell blocks) (rejected)
Forces every recipe through `[recipe.<name>.commands]` lists, which is verbose and awkward for any nontrivial recipe. The hybrid TOML+shell decision keeps declarative parts declarative and imperative parts imperative.

### Starlark / Rhai for recipes (rejected for v1)
Starlark would give us Bazel-grade composition. Not worth the dep weight + learning curve until there's evidence users want it. Revisit if shell-block boilerplate becomes painful.

### Build everything ourselves (no `cargo`/`go`/`clang` shellouts) (rejected)
Bazel-class scope. Not v1.

### Mtime-only caching (rejected)
Make's caching is a known footgun. We'd ship a regression vs. Cargo/Bazel. Hash for first-class recipes is the differentiator.

### Hash-only caching (rejected)
Forces every shell recipe to declare inputs/outputs, which is a regression vs. Make's "just put it in the recipe and we'll figure it out" UX. Mtime stays as the default for shell recipes.

### Auto-provision cross-toolchains in v1 (deferred)
Cross-toolchain provisioning is its own multi-month project (sysroots, glibc versions, macOS code-signing, NDK licensing). BYO + container opt-in covers 90% of real cases without us owning that complexity in v1.

### Runtime Makefile execution in v1 (deferred)
Implementing GNU Make semantics correctly is a multi-month project on its own (secondary expansion, `eval`, `define`, automatic vars, conditionals). Most users migrating want to *leave* Make. Ship import for v1; runtime mode in v0.2+.

---

## Things to revisit during implementation

### Dependency choices
- **`sled` vs. `sqlite` for cache index.** `sled` is pure-Rust, simpler, but unmaintained-ish. `rusqlite` is more standard and battle-tested. Start with `sled`; switch if it bites.
- **`logos` + `nom` for Makefile parsing.** Combo is overkill if Makefiles only use a few constructs. May simplify to a hand-rolled state machine.
- **`tracing` vs. `log`.** Going with `tracing` for spans; CLI subscribes via `tracing-subscriber`.
- **`clap` derive vs. builder.** Derive is cleaner; check that we can support `must <recipe>` (dynamic subcommands) — may need builder mode for that.

### Configuration ergonomics
- Glob expansion in `inputs`/`outputs` (`proto/**/*.proto`). Use `globset` or `ignore`. The `ignore` crate also gives us `.gitignore`-aware traversal which is probably what users want.
- Should `inputs` default to "everything in the project" if unset for a hash recipe? Probably no — too easy to over-invalidate. Better to require explicit declaration.

### Error UX
- Compare to Rust compiler / `ripgrep` errors. Aim for: file path, line number where applicable, what was expected, what to do next. No bare `Error: io::Error(...)`.
- Build a small "error catalog" doc as we go so we can audit error quality before v1.

### Cross-compile edge cases
- macOS → Linux cross is hard (glibc, `libSystem`). Document `cross = "container"` as the recommended path.
- Windows targets defer until someone needs them. The architecture supports it; we just don't test it in v1.
- `aarch64-apple-darwin` from `x86_64-apple-darwin` should "just work" with native tools.

### Recipe context (`BuildContext`)
- What does `ctx` contain? At minimum: project root, cache dir, current target, current profile, env, toolchain, dry-run flag. Keep it concrete and avoid making it a god-object.

---

## Future ideas (post-v1)

- **Watch mode.** `must watch build` re-runs on input changes. `notify` crate.
- **Remote cache.** S3 or HTTP backend with content-addressed storage. Big wins for CI.
- **Distributed execution.** Bazel-class. Probably never for the task-runner side; maybe for a `must-bazel-bridge` mode.
- **Recipe composition.** `extends = "base-rust-bin"` to reduce config duplication.
- **Conditional recipes.** `if = "host_os == 'macos'"` — but resist re-implementing Make's conditional spaghetti.
- **Plugin system.** Either dynamic-linked Rust plugins or wasm-based for sandboxing. wasm is probably the right answer.
- **IDE integration.** LSP-ish server providing recipe completion, "go to recipe definition", inline cache status.
- **`must init`.** Interactive scaffold based on detected languages in the project (looks for `Cargo.toml`, `go.mod`, `package.json`, etc.).
- **Recipe groups / tags.** `must build --tag fast` to run only recipes tagged "fast".

---

## Performance ideas to keep in mind

- **Avoid stat storms.** Mtime caching can issue a `stat()` per input; with thousands of inputs this gets slow. Batch via `walkdir` + cache results within a single invocation.
- **Avoid hashing the world.** For hash recipes, hash file contents in parallel using `rayon`; cache hash-of-content keyed by `(path, mtime, len)` so unchanged files don't get re-read.
- **Mind cold-start.** `must --version` / `must list` should be sub-100ms. That implies lazy initialization of the toolchain probe, cache opener, etc.
- **Process pool.** For shell recipes, reuse a long-lived shell process? Probably not worth it in v1 — measurable startup cost only on Windows. Defer.

---

## Naming / branding bikeshed

- Tool name: `must` (binary), `mustfile` (project / crate).
- Config file: `Mustfile.toml`. Capital M to mirror `Makefile`. The `.toml` extension makes editors happy.
- Tagline candidates:
  - "What Make should have been."
  - "The polyglot build tool."
  - "Build orchestration that knows your toolchain."
- Logo: TBD, probably "must" with a wrench/anvil. Defer until v1 ships.

---

## Risks / things that could go wrong

- **Scope creep on language modules.** Each new first-class module adds ongoing maintenance. Hold the line at Rust + Go + C/C++ for v1.
- **Container path is a UX cliff.** Users without Docker installed will get errors. Doctor should detect and explain. Document explicitly.
- **Makefile import will hit weird real-world cases.** The 80/20 cutoff is doing a lot of work. Test against a curated corpus of real OSS Makefiles before v1.
- **Cache directory bloat.** No eviction in v1 means heavy users will have multi-GB caches. `must doctor` should warn at thresholds.
- **Bazel/Buck users will compare us to those tools.** Need a clear "we're not trying to be Bazel" message in `OVERVIEW.md` and the README to avoid confused reviews.
