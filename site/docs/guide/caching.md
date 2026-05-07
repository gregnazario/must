# Caching

must uses intelligent caching to avoid redundant work. The strategy depends on the recipe type.

## Cache strategies

| Strategy | Used by | How it works |
|----------|---------|-------------|
| `hash` | rust-bin, go-bin, c-bin, ts-bin, etc. | Content hash of inputs + env + toolchain version |
| `mtime` | shell (default) | Modification time of input files |
| `never` | rust-test, go-test, py-test, etc. | Always re-run (test results should be fresh) |

## Content-hash caching

First-class recipe types (Rust, Go, C, TypeScript, etc.) use content-hashed caching:

1. Compute a SHA-256 hash from: recipe name, type, input file contents, env vars, toolchain version, extra flags
2. Look up the hash in the disk cache (`.mustfile/cache/`)
3. If found → cache hit, skip execution
4. If not found → execute, store the hash

This is **branch-switch safe** — switching branches changes file contents, which changes the hash.

### Cache key components

The hash is computed from:

- Recipe name
- Recipe type (e.g., `rust-bin`)
- Input file contents (sorted by path, each file SHA-256'd)
- Environment variables (sorted by key)
- Toolchain ID (e.g., `rustc 1.85.0`)
- Extra flags (profile, features, etc.)

### View cache key

```bash
must explain build
```

Shows the computed cache key and what went into it.

## mtime caching

Shell recipes default to mtime caching:

1. Check if any input file's mtime is newer than the last build
2. If newer → re-execute
3. If not → cache hit

### Opt into hash caching

```toml
[recipe.proto]
type   = "shell"
cache  = "hash"
inputs = ["proto/**/*.proto"]
script = "protoc --rust_out=src proto/*.proto"
```

## Phony recipes

Set `phony = true` to always re-run a recipe:

```toml
[recipe.clean]
type   = "shell"
phony  = true
script = "cargo clean"
```

## Cache management

```bash
must cache list                     # list cached entries
must cache du                       # disk usage
must cache invalidate build         # invalidate one recipe
must cache invalidate --all         # invalidate everything
```

## Cache location

Cache is stored in `.mustfile/cache/` relative to the Mustfile.toml directory.

Add to `.gitignore`:

```
.mustfile/
```
