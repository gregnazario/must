# All Recipe Types

must supports **41 recipe types** across **17 languages** plus **20 bridge tools**. Each recipe type knows how to invoke the right toolchain, compute cache keys, and handle errors.

## Quick reference

| Type | Language | Command | Cache |
|------|----------|---------|-------|
| `shell` | Any | Shell script | mtime (default) or hash |
| `rust-bin` | Rust | `cargo build -p <pkg>` | none |
| `rust-lib` | Rust | `cargo build --lib -p <pkg>` | none |
| `rust-test` | Rust | `cargo test -p <pkg>` | none |
| `go-bin` | Go | `go build` | none |
| `go-test` | Go | `go test` | none |
| `c-bin` | C | `cc -o <output> <sources>` | none |
| `c-lib` | C | `ar rcs` / `cc -shared` | none |
| `ts-bin` | TypeScript | `tsc --project` | none |
| `ts-check` | TypeScript | `tsc --noEmit` | none |
| `ts-lint` | TypeScript | `biome lint` | none |
| `npm` | JS/TS | `npm run <script>` | none |
| `py-bin` | Python | Python build | none |
| `py-test` | Python | `pytest` | none |
| `py-lint` | Python | `ruff check` + `mypy` | none |
| `zig-bin` | Zig | `zig build` | none |
| `zig-test` | Zig | `zig build test` | none |
| `java-bin` | Java | `./gradlew build` | none |
| `java-test` | Java | `./gradlew test` | none |
| `kotlin-bin` | Kotlin | `./gradlew build` | none |
| `kotlin-test` | Kotlin | `./gradlew test` | none |
| `swift-bin` | Swift | `swift build -c release` | none |
| `swift-test` | Swift | `swift test` | none |
| `dotnet-build` | .NET | `dotnet build` | none |
| `dotnet-test` | .NET | `dotnet test` | none |
| `dotnet-publish` | .NET | `dotnet publish` | none |
| `ruby-bin` | Ruby | `bundle install` + build | none |
| `ruby-test` | Ruby | `bundle exec rspec` | none |
| `dart-bin` | Dart | `dart compile exe` | none |
| `dart-test` | Dart | `dart test` | none |
| `elixir-build` | Elixir | `mix deps.get` + `mix compile` | none |
| `elixir-test` | Elixir | `mix test` | none |
| `flutter-build` | Flutter | `flutter build <platform>` | none |
| `flutter-test` | Flutter | `flutter test` | none |
| `nim-bin` | Nim | `nim c -d:release` | none |
| `nim-test` | Nim | `nim r --hints:off` | none |
| `docker-build` | Docker | `docker build` | none |
| `docker-push` | Docker | `docker push` | none |
| `precompiled-bin` | Any | Download + SHA-256 verify | hash |
| `plugin` | Lua | User-defined | mtime |
| `bridge` | Any | Delegates to existing tool | none |

`none` means must invokes the tool on every run and the tool's own incremental build decides what to do. See [Caching](../guide/caching.md).

## Common fields

All recipes support:

```toml
[recipe.<name>]
type    = "..."          # required
deps    = ["dep1"]       # optional dependencies
env     = { KEY = "val" }  # optional env vars
phony   = false           # optional: always re-run
```

## Per-language guides

See the individual recipe pages for detailed configuration, caching behavior, and examples:

- [Shell](shell.md) — generic shell scripts
- [Rust](rust.md) — cargo build/test/clippy
- [Go](go.md) — go build/test
- [C/C++](cc.md) — cc/c++ compilation
- [TypeScript](typescript.md) — tsc, biome, npm
- [Python](python.md) — pytest, ruff, mypy
- [Zig](zig.md) — zig build/test
- [Java/Kotlin](java-kotlin.md) — Gradle wrapper
- [Swift](swift.md) — swift build/test
- [.NET](dotnet.md) — dotnet build/test/publish
- [Ruby](ruby.md) — bundle + rspec
- [Dart](dart.md) — dart compile/test
- [Elixir](elixir.md) — mix compile/test
- [Flutter](flutter.md) — flutter build/test
- [Nim](nim.md) — nim compile/test
- [Docker](docker.md) — docker build/push
- [Precompiled Binaries](precompiled-bin.md) — download and cache prebuilt binaries
- [Bridge](bridge.md) — delegate to Make, npm, Gradle, Bazel, and 16 other tools
- [Plugins](../guide/plugins.md) — custom Lua recipe types
