# Cross-Platform Recipes

Mustfile supports writing shell recipes that work across operating systems.
On Unix (macOS, Linux) recipes use `sh -c`. On Windows they use `cmd /C`.

## Platform-specific scripts

Use `script` for the default (Unix) and `script_win` to override on Windows:

```toml
[recipe.clean]
type   = "shell"
phony  = true
script     = "rm -rf build"
script_win = "rmdir /s /q build"
```

When `script_win` is set and running on Windows, it takes precedence over `script`.
On all other platforms, `script` is used. If `script_win` is not set, `script` is
used everywhere.

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
script     = "mkdir -p build && cc -o build/app src/main.c"
script_win = "if not exist build mkdir build && cl /Fe:build/app.exe src/main.c"

[recipe.clean]
type   = "shell"
phony  = true
script     = "rm -rf build"
script_win = "rmdir /s /q build"

[recipe.test]
type   = "shell"
deps   = ["build"]
phony  = true
script     = "./build/app --test"
script_win = "build\\app.exe --test"
```

## How it works

| Platform | Shell         | Field used          |
|----------|---------------|---------------------|
| macOS    | `sh -c`       | `script`            |
| Linux    | `sh -c`       | `script`            |
| Windows  | `cmd /C`      | `script_win` or `script` |

## Tips

- **Path separators**: Use `/` in `script` and `\\` in `script_win` for path separators.
- **Environment variables**: `$VAR` syntax works in `sh`, `%VAR%` works in `cmd`.
- **Language recipes don't need this**: Recipes like `rust-bin`, `go-bin`, `py-bin` etc.
  invoke language-specific tools (cargo, go, pip) that are already cross-platform.
  Only `shell` and `npm` recipes need `script_win`.
- **CI**: You can test Windows scripts in CI by running `must build` on a Windows runner
  alongside your Unix runners.

## See also

- [Getting Started](getting-started.md)
- [Config Reference](config-reference.md)
- [Cross Compilation](cross-compilation.md)
