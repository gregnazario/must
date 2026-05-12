# Bridge Recipes

Bridge recipes delegate to existing build tools — no rewriting required. `must` detects your project's build system and runs the right commands.

## Auto-detect mode

When no `Mustfile.toml` exists, `must` scans the project root for known build files and generates virtual recipes automatically:

```bash
cd my-makefile-project   # has a Makefile but no Mustfile.toml
must build               # runs: make
must test                # runs: make test
must list                # shows auto-detected recipes
```

Output:

```
(auto-detected from build files — no Mustfile.toml found)
NAME                 TYPE         DEPS
build                bridge
test                 bridge
clean                bridge
lint                 bridge
fmt                  bridge
```

## Multi-tool projects

When multiple build files are detected, `must` assigns the first tool to the standard verbs and prefixes the rest:

```
NAME                 TYPE         DEPS
build                bridge       → make
test                 bridge       → make
npm-build            bridge       → npm run build
npm-test             bridge       → npm run test
npm-lint             bridge       → npm run lint
```

## Explicit configuration

Use `type = "bridge"` in `Mustfile.toml` for explicit control:

```toml
[recipe.build]
type    = "bridge"
package = "make"
script  = "make build"

[recipe.test]
type    = "bridge"
package = "npm"
script  = "npm test"
```

### Required fields

| Field | Purpose |
|-------|---------|
| `type` | `"bridge"` |
| `package` | Tool name (e.g. `make`, `npm`, `gradle`) |
| `script` | Full command to execute |

Bridge recipes always use `cache = "none"` since the delegate tool manages its own caching.

## Supported tools

must detects **20 build tools**. Each knows its indicator files and default targets:

| Tool | Detects | `build` | `test` | `clean` | `lint` | `fmt` |
|------|---------|---------|--------|---------|--------|-------|
| **Make** | `Makefile`, `GNUmakefile` | `make` | `make test` | `make clean` | `make lint` | `make fmt` |
| **npm** | `package.json` | `npm run build` | `npm run test` | — | `npm run lint` | `npm run format` |
| **Gradle** | `build.gradle`, `build.gradle.kts` | `gradle build` | `gradle test` | `gradle clean` | `gradle check` | `gradle spotlessApply` |
| **Maven** | `pom.xml` | `mvn compile` | `mvn test` | `mvn clean` | `mvn verify` | `mvn fmt:format` |
| **Rake** | `Rakefile` | `rake build` | `rake test` | `rake clean` | `rake lint` | `rake format` |
| **Invoke** | `tasks.py` | `invoke build` | `invoke test` | — | `invoke lint` | `invoke format` |
| **CMake** | `CMakeLists.txt` | `cmake --build build` | `ctest --test-dir build` | — | — | — |
| **Cargo Make** | `Makefile.toml` | `cargo make build` | `cargo make test` | `cargo make clean` | `cargo make lint` | `cargo make format` |
| **Ant** | `build.xml` | `ant build` | `ant test` | `ant clean` | — | — |
| **Just** | `justfile`, `Justfile` | `just build` | `just test` | `just clean` | `just lint` | `just fmt` |
| **Bazel** | `WORKSPACE`, `MODULE.bazel` | `bazel build //...` | `bazel test //...` | `bazel clean` | — | — |
| **Buck2** | `BUCK` | `buck2 build //...` | `buck2 test //...` | `buck2 clean` | — | — |
| **Pants** | `pants.toml` | `pants package ::` | `pants test ::` | `pants clean-all` | `pants lint ::` | `pants fmt ::` |
| **Meson** | `meson.build` | `meson compile -C builddir` | `meson test -C builddir` | — | — | — |
| **Yarn** | `yarn.lock` | `yarn build` | `yarn test` | — | `yarn lint` | `yarn format` |
| **pnpm** | `pnpm-lock.yaml` | `pnpm build` | `pnpm test` | — | `pnpm lint` | `pnpm format` |
| **Bun** | `bun.lockb`, `bun.lock` | `bun run build` | `bun run test` | — | `bun run lint` | `bun run format` |
| **sbt** | `build.sbt` | `sbt compile` | `sbt test` | `sbt clean` | — | — |
| **Gulp** | `gulpfile.js`, `gulpfile.mjs`, `gulpfile.ts` | `gulp build` | `gulp test` | `gulp clean` | — | — |
| **Nx** | `nx.json` | `nx run-many --target=build --all` | `nx run-many --target=test --all` | — | `nx run-many --target=lint --all` | `nx format` |

## How it works

1. **Detection**: `must` checks for indicator files in the project root
2. **Recipe generation**: Each detected tool generates default recipes (build, test, clean, lint, fmt)
3. **Execution**: Bridge recipes run the delegate command via shell — the tool handles everything
4. **No caching**: Bridge recipes use `cache = "none"` since the delegate tool manages its own incremental builds

## When to use bridge vs. native recipes

| Scenario | Use |
|----------|-----|
| Existing project, zero config | Bridge (auto-detect) |
| Mixed-language monorepo with Gradle + npm | Bridge (explicit) |
| New Rust/Go/Python project | Native recipe types (`rust-bin`, `go-bin`, etc.) |
| Need caching, cross-compilation, or toolchain detection | Native recipe types |
| Legacy build system you can't change | Bridge |

Native recipe types offer caching, cross-compilation, and toolchain version detection. Bridge is for when you want the unified `must` interface without rewriting your existing build.
