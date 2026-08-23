# Precompiled Binaries

The `precompiled-bin` recipe type downloads prebuilt binaries and caches them with SHA-256 verification.

## Quick start

Download and cache a prebuilt tool from a GitHub release:

```toml
# Mustfile.toml
[project]
name = "my-project"
version = "0.1.0"

[recipe.rg]
type   = "precompiled-bin"
url    = "https://github.com/BurntSushi/ripgrep/releases/download/14.1.1/ripgrep-14.1.1-x86_64-unknown-linux-musl.tar.gz"
sha256 = "5a78ec8fbed3c1e55e1a19faacf4941b9c3e0d13b6e7e3c6b6b0e94be22f01b6"
output = ".tools/rg"
```

```bash
$ must build
✓ [#################] 1/1 (starting...)
  ✓ rg  downloading https://github.com/BurntSushi/ripgrep/releases/...
1 built, 0 cached, 0 failed — 1.8s

$ must build
✓ [#################] 1/1 (starting...)
  ✓ rg  (cached)
0 built, 1 cached, 0 failed — 12ms
```

## When to use

- Downloading tools your project depends on (e.g., `protoc`, `buf`, `dart-sass`)
- Pinning CLI versions across your team
- Avoiding system-level installs in CI

## Configuration

```toml
[recipe.protoc]
type   = "precompiled-bin"
url    = "https://github.com/protocolbuffers/protobuf/releases/download/v28.0/protoc-28.0-linux-x86_64.zip"
sha256 = "abc123..."
output = "bin/protoc"
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | HTTPS URL to download |
| `sha256` | string | Recommended | Expected SHA-256 hex digest |
| `output` | string | Yes | Output path relative to project root |

### Security

- **HTTPS only** — `http://` URLs are rejected
- **Path traversal protection** — `output` cannot contain `..`
- **SHA-256 verification** — streamed chunked verification before finalizing the file

## How it works

1. Downloads to a `.tmp` file using streaming I/O (low memory usage)
2. Verifies the SHA-256 digest of the download against `sha256` (chunked, O(1) heap)
3. For raw binary URLs, atomically renames `.tmp` to the output path.
   For archive URLs (`.tar.gz`, `.tgz`, `.zip`), extracts the archive and
   installs one binary from it
4. On cache hit, skips download entirely

### Archive handling

URLs ending in `.tar.gz`, `.tgz`, or `.zip` are extracted after the SHA-256
check (the digest pins the **archive**, not the binary inside). The binary to
install is chosen automatically:

1. a file whose name exactly matches the recipe name (e.g. `[recipe.protoc]` → `bin/protoc`)
2. otherwise, the archive's single executable file (falling back to its single
   non-document file when the archive carries no executable bits)

Ambiguous archives produce an error listing the candidates — name the recipe
after the binary or point `url` at the raw binary instead. For archive URLs
the installed binary is recorded alongside the output as `<output>.mustsha256`
so a bumped pin still triggers re-download.

## Example: multiple tools

```toml
[project]
name = "my-app"
version = "0.1.0"

[recipe.protoc]
type   = "precompiled-bin"
url    = "https://github.com/protocolbuffers/protobuf/releases/download/v28.0/protoc-28.0-linux-x86_64.zip"
sha256 = "abc123..."
output = ".tools/protoc"

[recipe.buf]
type   = "precompiled-bin"
url    = "https://github.com/bufbuild/buf/releases/download/v1.40.0/buf-Linux-x86_64"
sha256 = "def456..."
output = ".tools/buf"

[recipe.generate]
type    = "shell"
deps    = ["protoc"]
inputs  = ["proto/*.proto"]
outputs = ["src/generated/*.rs"]
script  = "protoc --rust_out=src proto/*.proto"
```

## See also

- [Caching](../guide/caching.md)
- [Config Reference](../guide/config-reference.md)
