# Precompiled Binaries

The `precompiled-bin` recipe type downloads prebuilt binaries and caches them with SHA-256 verification.

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
2. Verifies SHA-256 digest against `sha256` field (chunked, O(1) heap)
3. Atomically renames `.tmp` to the final output path
4. On cache hit, skips download entirely

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
