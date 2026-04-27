# Mustfile — Overview

> A Rust-implemented Makefile replacement that automates polyglot builds with consistent target names, automatic cross-compilation, and unified handling of common configurations.

## What is Mustfile?

Mustfile is a build orchestrator with a single binary (`must`) and a TOML configuration file (`Mustfile.toml`). It sits between pure task runners (Make, Just, Task) and full build systems (Bazel, Buck2):

- **Task runner where it should be.** Generic recipes are shell blocks. No DSL to learn beyond TOML.
- **Build system where it counts.** First-class recipe types for **Rust**, **Go**, and **C/C++** in v1 — Mustfile understands their toolchains, so cross-compilation, env vars, and linkers are wired up automatically.
- **Consistent verbs across languages.** `must build`, `must test`, `must release` mean the same thing whether the recipe builds Rust, Go, or C.
- **Pragmatic caching.** First-class recipes use content-hashed caching (reproducible, branch-switch-safe). Generic shell recipes default to mtime — and can opt into hashing.
- **Migration-friendly.** `must import` reads an existing `Makefile` and emits a starter `Mustfile.toml` plus a report listing anything that needs human review.

## Why does it exist?

Make is everywhere, but it is also brittle in well-known ways:

| Pain | What Mustfile does |
|---|---|
| mtime-based rebuilds break on branch switches | Content-hashed caching for first-class recipes |
| No language awareness — every Makefile reinvents Rust/Go/C wiring | First-class recipe types that know each toolchain |
| Cross-compilation is manual and per-project | Automatic toolchain resolution + container opt-in |
| Env vars, linkers, sysroots scattered everywhere | Centralized in `[env]`, `[targets]`, `[recipe.<name>.cross]` |
| Tab-significant DSL with arcane expansion rules | TOML structure + plain shell recipe bodies |
| Hard to introspect | `must list`, `must graph`, `must --explain` |

Modern alternatives either commit to a single language (Cargo) or carry a steep adoption curve and ecosystem buy-in (Bazel, Buck2). Mustfile aims for the pragmatic middle.

## Quick example

```toml
# Mustfile.toml
[project]
name = "myapp"

[env]
RUST_LOG = "info"

[targets]
release = ["x86_64-linux-gnu", "aarch64-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"]

[recipe.build]
type = "rust-bin"
package = "myapp"
features = ["cli"]

[recipe.test]
type = "rust-test"
package = "myapp"

[recipe.serve]
type = "go-bin"
package = "./cmd/server"
ldflags = "-s -w"

[recipe.release]
type  = "shell"
deps  = ["build", "test"]
script = "tar czf myapp.tar.gz target/release/myapp"
```

```sh
must build                                  # builds for host
must build --target aarch64-linux-gnu       # cross-compiles
must release --profile release              # multi-target release w/ deps
must list                                   # show all recipes
must graph                                  # render DAG
must --explain build                        # why will/won't this rebuild?
must import --makefile Makefile             # convert from Make
```

## How does it compare?

|                       | Make | Just | Cargo | Bazel | **Mustfile** |
|-----------------------|:---:|:---:|:---:|:---:|:---:|
| Polyglot              | ✅ | ✅ | ❌ | ✅ | ✅ |
| Language-aware        | ❌ | ❌ | ✅ | ✅ | ✅ |
| Auto cross-compile    | ❌ | ❌ | partial | ✅ | ✅ |
| Content-hash caching  | ❌ | ❌ | ✅ | ✅ | ✅ (first-class recipes) |
| TOML / no DSL         | ❌ | ❌ | ✅ | ❌ | ✅ |
| Imports Makefiles     | n/a | ❌ | ❌ | ❌ | ✅ |
| Single static binary  | ✅ | ✅ | ✅ | ❌ | ✅ |
| Setup overhead        | low | low | low | high | low |

## Status & roadmap

- **v1 (in design):** Rust + Go + C/C++ first-class, BYO toolchains + container opt-in, Makefile one-shot import, hash + mtime caching, parallel scheduler.
- **v0.2+:** runtime `make`-mode (`must -f Makefile`), toolchain auto-provisioning, remote cache.
- **Later:** JS/TS recipe module, plugin system, IDE integration.

## Where to go next

- [`DESIGN.md`](./DESIGN.md) — full technical design with diagrams, abstractions, and module breakdown.
- [`TASKS.md`](./TASKS.md) — milestone-by-milestone task list.
- [`SCRATCHPAD.md`](./SCRATCHPAD.md) — open questions, alternatives considered, future ideas.
