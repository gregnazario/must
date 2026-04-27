# Mustfile

> A polyglot build orchestrator with first-class support for Rust, Go, and C/C++. One binary, one config, consistent verbs across languages.

Mustfile sits between pure task runners (Make, Just) and full build systems (Bazel, Buck2):

- **Consistent verbs:** `must build`, `must test`, `must release` — same commands regardless of language
- **Language-aware:** First-class recipe types for Rust, Go, and C/C++ handle toolchains and cross-compilation automatically
- **Pragmatic caching:** Content-hash caching for first-class recipes; mtime for shell recipes
- **Migration-friendly:** `must import` converts existing Makefiles

## Documentation

- [Overview](docs/OVERVIEW.md) — What Mustfile is and why it exists
- [Design](docs/DESIGN.md) — Architecture, abstractions, and execution model
- [Tasks](docs/TASKS.md) — Milestone roadmap

## Status

Pre-release. See [TASKS.md](docs/TASKS.md) for the roadmap.

## License

MIT — see [LICENSE](LICENSE).
