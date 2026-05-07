# All Recipe Types

must supports **41 recipe types** across **17 languages**. Each recipe type knows how to invoke the right toolchain, compute cache keys, and handle errors.

## Quick reference

| Type | Language | Command | Cache |
|------|----------|---------|-------|
| `shell` | Any | Shell script | mtime (default) or hash |
| `rust-bin` | Rust | `cargo build -p <pkg>` | hash |
| `rust-lib` | Rust | `cargo build --lib -p <pkg>` | hash |
| `rust-test` | Rust | `cargo test -p <pkg>` | never |
| `go-bin` | Go | `go build` | hash |
| `go-test` | Go | `go test` | never |
| `c-bin` | C | `cc -o <output> <sources>` | hash |
| `c-lib` | C | `ar rcs` / `cc -shared` | hash |
| `ts-bin` | TypeScript | `tsc --project` | hash |
| `ts-check` | TypeScript | `tsc --noEmit` | never |
| `ts-lint` | TypeScript | `biome lint` | never |
| `npm` | JS/TS | `npm run <script>` | hash |
| `py-bin` | Python | Python build | hash |
| `py-test` | Python | `pytest` | never |
| `py-lint` | Python | `ruff check` + `mypy` | never |
| `zig-bin` | Zig | `zig build` | hash |
| `zig-test` | Zig | `zig build test` | never |
| `java-bin` | Java | `./gradlew build` | hash |
| `java-test` | Java | `./gradlew test` | never |
| `kotlin-bin` | Kotlin | `./gradlew build` | hash |
| `kotlin-test` | Kotlin | `./gradlew test` | never |
| `swift-bin` | Swift | `swift build -c release` | hash |
| `swift-test` | Swift | `swift test` | never |
| `dotnet-build` | .NET | `dotnet build` | hash |
| `dotnet-test` | .NET | `dotnet test` | never |
| `dotnet-publish` | .NET | `dotnet publish` | hash |
| `ruby-bin` | Ruby | `bundle install` + build | hash |
| `ruby-test` | Ruby | `bundle exec rspec` | never |
| `dart-bin` | Dart | `dart compile exe` | hash |
| `dart-test` | Dart | `dart test` | never |
| `elixir-build` | Elixir | `mix deps.get` + `mix compile` | hash |
| `elixir-test` | Elixir | `mix test` | never |
| `flutter-build` | Flutter | `flutter build <platform>` | hash |
| `flutter-test` | Flutter | `flutter test` | never |
| `nim-bin` | Nim | `nim c -d:release` | hash |
| `nim-test` | Nim | `nim r --hints:off` | never |
| `docker-build` | Docker | `docker build` | hash |
| `docker-push` | Docker | `docker push` | never |
| `plugin` | Lua | User-defined | mtime |

## Common fields

All recipes support:

```toml
[recipe.<name>]
type    = "..."          # required
deps    = ["dep1"]       # optional dependencies
env     = { KEY = "val" }  # optional env vars
workdir = "subdir/"      # optional working directory
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
