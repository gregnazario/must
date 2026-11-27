# Caching

must uses caching to avoid redundant work. The strategy depends on the recipe type.

## Cache strategies

| Strategy | Used by | How it works |
|----------|---------|-------------|
| `hash` | `shell` recipes with `cache = "hash"`, `precompiled-bin` | Content hash of the declared inputs, env and flags |
| `mtime` | `shell` (default) | Modification times of the declared inputs and outputs |
| `none` | every other recipe type (`rust-bin`, `go-bin`, `c-bin`, `ts-bin`, all test recipes, …) | The tool is invoked on every run and decides what to rebuild |

## Tool-backed recipes always run

Recipes such as `rust-bin`, `go-bin` or `elixir-build` hand the work to a tool that already tracks its own dependencies: cargo, go, mix and their peers rebuild what changed and do nothing when nothing did. must therefore invokes the tool on every `must build` and never skips one of these recipes because of a stored cache entry. A key built from the recipe's name, environment and toolchain version knows nothing about the source files, so skipping on it would keep serving stale, or deleted, artifacts after an edit.

`cache`, `phony`, `inputs` and `outputs` have no effect on these recipe types. `must explain` says so when `cache` is set:

```
$ must explain build
Strategy: none (`cache` is ignored by this recipe type)
Status:    ALWAYS RUNS (not cached)
```

To skip a step that a tool does not make incremental (a download, a code generator, a packaging step), put it in a `shell` recipe with declared `inputs` and `outputs`.

## Content-hash caching

Shell recipes opt into content-hash caching with `cache = "hash"`:

```toml
[recipe.proto]
type   = "shell"
cache  = "hash"
inputs = ["proto/**/*.proto"]
outputs = ["src/generated.rs"]
script = "protoc --rust_out=src proto/*.proto"
```

1. Compute a SHA-256 hash from the recipe name, type, input file contents, env vars and script
2. Look up the hash in the disk cache (`.must/cache/`)
3. If found and every declared output is present (or can be restored from the cache), skip execution
4. Otherwise execute and store the hash together with the outputs

This is **branch-switch safe**: switching branches changes file contents, which changes the hash.

### Cache key components

- Recipe name
- Recipe type (`shell`)
- Input file contents (sorted by path, each file SHA-256'd)
- Environment variables (sorted by key)
- The script

### View cache key

```bash
must explain build
```

Shows the computed cache key and what went into it.

## mtime caching

Shell recipes default to mtime caching:

1. Compare the modification times of the declared inputs and outputs
2. If an input is newer than an output, or an output is missing, re-execute
3. Otherwise skip execution

## Phony recipes

Set `phony = true` on a `shell` recipe to always re-run it:

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

Cache is stored in `.must/cache/` relative to the Mustfile.toml directory.

Add to `.gitignore`:

```
.must/
```
