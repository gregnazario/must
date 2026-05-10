# Cross-Platform Recipes

Mustfile supports writing shell recipes that work across macOS, Linux, FreeBSD, and Windows.
On Unix systems recipes use `sh -c`. On Windows they use `cmd /C`.

## Quick: Windows override

Use `script` for the default and `script_win` for Windows:

```toml
[recipe.clean]
type   = "shell"
phony  = true
script     = "rm -rf build"
script_win = "rmdir /s /q build"
```

## Per-OS scripts

For finer control, use the `scripts` table to override per operating system:

```toml
[recipe.build]
type   = "shell"
script = "make"

[recipe.build.scripts]
macos   = "make -j$(sysctl -n hw.ncpu)"
linux   = "make -j$(nproc)"
freebsd = "gmake -j$(sysctl -n hw.ncpu)"
win     = "nmake"
```

### Resolution order

The `scripts` table is checked in order of specificity. The first match wins:

| Platform | Checked in order |
|----------|-----------------|
| macOS | `scripts.macos` → `scripts.unix` → `script` |
| Linux (Ubuntu) | `scripts.linux.ubuntu` → `scripts.linux` → `scripts.unix` → `script` |
| Linux (Alpine) | `scripts.linux.alpine` → `scripts.linux` → `scripts.unix` → `script` |
| Linux (other) | `scripts.linux.<distro>` → `scripts.linux` → `scripts.unix` → `script` |
| FreeBSD | `scripts.freebsd` → `scripts.bsd` → `scripts.unix` → `script` |
| NetBSD / OpenBSD | `scripts.netbsd` / `scripts.openbsd` → `scripts.bsd` → `scripts.unix` → `script` |
| Windows | `scripts.win` → `script_win` → `script` |

The distro ID is read from `ID=` in `/etc/os-release` (e.g., `ubuntu`, `debian`, `alpine`, `arch`, `fedora`, `amzn`).

The `scripts` table takes priority over `script_win` and `script`.

### Distro-specific example

```toml
[recipe.install-deps]
type   = "shell"
script = "make deps"

[recipe.install-deps.scripts]
"linux.ubuntu"  = "apt-get install -y libssl-dev"
"linux.alpine"  = "apk add openssl-dev"
"linux.fedora"  = "dnf install openssl-devel"
"linux.arch"    = "pacman -S openssl"
"linux.amzn"    = "yum install openssl-devel"
linux           = "make deps"
macos           = "brew install openssl"
```

### Available keys

| Key | Matches |
|-----|---------|
| `macos` | macOS only |
| `linux` | Linux only |
| `linux.ubuntu` | Ubuntu |
| `linux.debian` | Debian |
| `linux.alpine` | Alpine Linux |
| `linux.arch` | Arch Linux |
| `linux.fedora` | Fedora |
| `linux.amzn` | Amazon Linux |
| `linux.<distro>` | Any distro ID from `/etc/os-release` |
| `freebsd` | FreeBSD only |
| `netbsd` | NetBSD only |
| `openbsd` | OpenBSD only |
| `win` | Windows only |
| `unix` | Any Unix (macOS, Linux, FreeBSD, NetBSD, OpenBSD, ...) |
| `bsd` | FreeBSD, NetBSD, OpenBSD |

## Full example

```toml
[project]
name = "my-app"
version = "1.0.0"

[recipe.build]
type    = "shell"
cache   = "hash"
inputs  = ["src/**/*.c"]
outputs = ["build/app"]
script  = "mkdir -p build && cc -o build/app src/main.c"

[recipe.build.scripts]
macos   = "mkdir -p build && cc -o build/app src/main.c"
linux   = "mkdir -p build && gcc -o build/app src/main.c"
freebsd = "mkdir -p build && clang -o build/app src/main.c"
win     = "if not exist build mkdir build && cl /Fe:build/app.exe src/main.c"

[recipe.clean]
type  = "shell"
phony = true

[recipe.clean.scripts]
macos   = "rm -rf build"
linux   = "rm -rf build"
freebsd = "rm -rf build"
win     = "rmdir /s /q build"

[recipe.test]
type   = "shell"
deps   = ["build"]
phony  = true
script = "./build/app --test"

[recipe.test.scripts]
win = "build\\app.exe --test"
```

## Tips

- **Path separators**: Use `/` in Unix scripts, `\\` in Windows scripts.
- **Environment variables**: `$VAR` in `sh`, `%VAR%` in `cmd`.
- **Language recipes don't need this**: Recipes like `rust-bin`, `go-bin`, `py-bin` etc.
  invoke language-specific tools that are already cross-platform.
  Only `shell` and `npm` recipes typically need platform overrides.
- **CI**: Test Windows scripts on a Windows runner alongside your Unix runners.
- **Fallback**: Always set `script` as the default — it's used when no platform matches.

## See also

- [Getting Started](getting-started.md)
- [Config Reference](config-reference.md)
- [Cross Compilation](cross-compilation.md)
- [Troubleshooting](troubleshooting.md)
