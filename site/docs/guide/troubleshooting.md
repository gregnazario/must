# Troubleshooting

Common issues and how to resolve them.

## `must doctor` — Start here

Run `must doctor` to check which toolchains are installed and get installation hints:

```
must doctor — environment health check

  ✓ cargo         1.78.0
  ✗ go (optional) Install from https://go.dev/dl/
  ✓ cc/gcc        Apple clang 15.0
  ✗ tsc (optional) npm install -g typescript
  ...
```

Items marked `(optional)` are only needed for recipe types that use them.

---

## Errors

### `unknown recipe 'name'`

You referenced a recipe name that doesn't exist in your `Mustfile.toml`.

**Fix:** Check spelling and run `must list` to see all available recipes.

### `cycle detected in recipe graph: a → b → a`

Two or more recipes form a circular dependency.

**Fix:** Remove one of the `deps` entries that creates the cycle.

### `recipe 'name' failed with exit code 1`

The recipe's command returned a non-zero exit code.

**Fix:**
1. Run `must log <name>` to see the full output
2. Run the command directly in your shell to reproduce
3. Check working directory — recipes run in the project root (where `Mustfile.toml` lives)

### `tool not found: cargo`

The required toolchain is not installed or not on `PATH`.

**Fix:** Install the tool and verify with `must doctor`. Common installations:

| Tool | Install |
|------|---------|
| `cargo` | [rustup.rs](https://rustup.rs) |
| `go` | [go.dev/dl](https://go.dev/dl/) |
| `cc` / `gcc` | Xcode CLI tools (macOS), `build-essential` (Linux) |
| `tsc` | `npm install -g typescript` |
| `biome` | `npm install -g @biomejs/biome` |
| `python3` | [python.org](https://python.org) or `brew install python` |
| `uv` | [docs.astral.sh/uv](https://docs.astral.sh/uv/) |
| `zig` | [ziglang.org](https://ziglang.org/) |
| `docker` | [docker.com](https://docker.com) |
| `swift` | Xcode (macOS) or [swift.org](https://swift.org) |
| `dotnet` | [dot.net](https://dot.net) |
| `dart` | [dart.dev](https://dart.dev) |
| `nim` | `choosenim` or [nim-lang.org](https://nim-lang.org) |
| `elixir` / `mix` | [elixir-lang.org](https://elixir-lang.org) |
| `flutter` | [flutter.dev](https://flutter.dev) |
| `gradle` | Build scan or [gradle.org](https://gradle.org) |
| `bundle` / `ruby` | [ruby-lang.org](https://ruby-lang.org) |

### `toolchain not found for target 'x86_64-unknown-linux-gnu'`

Cross-compilation target requires a specific linker or toolchain.

**Fix:**
1. Install the cross-compilation toolchain (e.g., `apt install gcc-x86-64-linux-gnu`)
2. Configure the linker in your Mustfile:

```toml
[recipe.build.cross]
"x86_64-unknown-linux-gnu" = { linker = "x86_64-linux-gnu-gcc" }
```

Or use container-based cross-compilation:

```toml
[recipe.build.cross]
"x86_64-unknown-linux-gnu" = { cross = "container" }
```

### `config error in Mustfile.toml: ...`

TOML parsing or validation error.

**Fix:** Check the error message for the specific issue. Common causes:
- Missing `type` field on a recipe
- Invalid recipe type name
- Missing required field (e.g., `package` for `rust-bin`)

---

## Caching issues

### Recipe always re-runs (never caches)

**Check:**
- Is `phony = true` set? This forces re-execution every time.
- Is `cache = "none"` set?
- For mtime caching: do your `outputs` exist? If outputs are missing, the recipe always re-runs.

### Cache is stale after switching branches

Hash-based caching is branch-switch safe because it uses content hashes. If you're using mtime caching, branch switches can cause false rebuilds because file timestamps change.

**Fix:** Use `cache = "hash"` for expensive recipes:

```toml
[recipe.build]
type   = "shell"
cache  = "hash"
inputs = ["src/**/*.rs"]
script = "cargo build"
```

### Clearing cache

```bash
must cache invalidate build     # clear one recipe
must cache invalidate --all     # clear everything
must clean --cache              # remove outputs + cache
```

---

## Platform scripts

### Script override not working

If a `scripts` table key isn't matching, check the resolution order:

1. `scripts.linux.<distro>` (Linux only, distro from `/etc/os-release`)
2. `scripts.linux` / `scripts.macos` / `scripts.freebsd` (exact OS)
3. `scripts.bsd` (FreeBSD/NetBSD/OpenBSD) or `scripts.unix` (any Unix)
4. `scripts.win` (Windows)
5. `script_win` (Windows shorthand)
6. `script` (default fallback)

Run `must explain <recipe>` to see which script would be executed.

### `/etc/os-release` not found

Distro-level keys (`linux.ubuntu`, `linux.alpine`) only work on Linux systems with `/etc/os-release`. On other platforms or minimal containers, use `linux` as a catch-all.

---

## Performance

### Builds are slow

1. **Use parallelism:** `must build -j4` (default is number of CPUs)
2. **Check cache hits:** `must outdated` shows which recipes are fresh vs stale
3. **Use hash caching** for expensive shell recipes (codegen, protoc, etc.)

### `must` itself is slow to start

The `must` binary is a single compiled Rust binary. If startup is slow, check:
- Disk I/O on the cache directory (`.must/cache/`)
- Number of input file globs being expanded

---

## File watching

### `must watch` doesn't detect changes

The watcher uses `notify` which relies on OS file system events. Known limitations:
- NFS mounts and some CI environments don't emit events
- Very large directories may have delayed detection
- On macOS, `FSEvents` has a latency of a few seconds

---

## See also

- [CLI Reference](cli-reference.md) — all commands and flags
- [Caching](caching.md) — hash vs mtime strategies
- [Config Reference](config-reference.md) — full Mustfile.toml schema
