# Cross-Compilation

must supports cross-compilation with automatic toolchain resolution.

## Defining targets

```toml
[targets]
default = ["x86_64-unknown-linux-gnu"]
release = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
```

## Building for a target

```bash
must build --target aarch64-unknown-linux-gnu
must build --target release   # builds all targets in the group
```

## Per-recipe cross-compilation config

Override linker, toolchain, or container settings per target:

```toml
[recipe.cli]
type    = "rust-bin"
package = "cli"

[recipe.cli.cross]
"x86_64-unknown-linux-gnu" = {}
"aarch64-unknown-linux-gnu" = { linker = "aarch64-linux-gnu-gcc" }
"x86_64-pc-windows-msvc" = { cross = "container" }
```

### Cross-compilation fields

| Field | Description |
|-------|-------------|
| `linker` | Custom linker for this target |
| `cross` | `"container"` to use cross-rs Docker containers |
| `ar` | Custom archiver |
| `sysroot` | Custom sysroot path |

## Language-specific behavior

### Rust

- Uses `cargo build --target <triple>`
- Supports cross-rs containers via `cross = "container"`
- Automatic linker configuration from target triple

### Go

- Sets `GOOS` and `GOARCH` from the target triple
- Supports all standard Go cross-compilation targets

### C/C++

- Uses the target triple to select a cross-compiler
- Example: `aarch64-unknown-linux-gnu` → `aarch64-linux-gnu-gcc`

## Target triple format

must uses LLVM target triples:

```
<arch>-<vendor>-<os>-<abi>
```

Common triples:

| Triple | Platform |
|--------|----------|
| `x86_64-unknown-linux-gnu` | Linux x86_64 |
| `aarch64-unknown-linux-gnu` | Linux ARM64 |
| `x86_64-apple-darwin` | macOS Intel |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-pc-windows-msvc` | Windows x86_64 |
