# Plugins

Extend must with Lua scripts. Plugins have access to shell execution, file I/O, glob, environment variables, and logging.

## Plugin location

Place `.lua` files in `.must/plugins/`:

```
.must/
  plugins/
    protoc.lua
    codegen.lua
```

## Using a plugin

```toml
[recipe.codegen]
type   = "plugin"
plugin = "protoc"    # matches protoc.lua
```

## Plugin structure

A plugin must define an `execute` function:

```lua
function execute(ctx)
    -- ctx.profile: current profile name
    -- ctx.target: current target triple
    -- ctx.project_root: project root path
    -- ctx.env: table of environment variables

    return {
        stdout = "output message",
        stderr = "",
        success = true,
    }
end
```

## Optional functions

### `deps()`

Declare dependencies:

```lua
function deps()
    return { "build", "codegen" }
end
```

### `inputs(ctx)`

Return input file paths for caching:

```lua
function inputs(ctx)
    return { "proto/service.proto", "proto/types.proto" }
end
```

### `outputs(ctx)`

Return output file paths:

```lua
function outputs(ctx)
    return { "src/proto/mod.rs" }
end
```

### `cache_key(ctx)`

Custom cache key (default: script content hash):

```lua
function cache_key(ctx)
    return "custom-" .. ctx.profile
end
```

## Built-in stdlib

| Function | Signature | Description |
|----------|-----------|-------------|
| `shell_exec` | `shell_exec(cmd) → table` | Execute shell command. Returns `{success, exit_code, stdout, stderr}` |
| `read_file` | `read_file(path) → string` | Read file contents |
| `write_file` | `write_file(path, content)` | Write to file |
| `file_exists` | `file_exists(path) → bool` | Check if file exists |
| `mkdir` | `mkdir(path)` | Create directory (recursive) |
| `glob` | `glob(pattern) → table` | Match file paths (returns 1-indexed table) |
| `env_get` | `env_get(key) → string?` | Get environment variable |
| `set_env` | `set_env(key, value)` | Set environment variable |
| `log_info` | `log_info(msg)` | Log info message |
| `log_warn` | `log_warn(msg)` | Log warning |

## Example: Protocol buffer codegen

```lua
-- .must/plugins/protoc.lua

function inputs(ctx)
    return glob(ctx.project_root .. "/proto/*.proto")
end

function outputs(ctx)
    return { "src/proto/mod.rs", "src/proto/types.rs" }
end

function execute(ctx)
    local protos = glob(ctx.project_root .. "/proto/*.proto")
    local count = 0

    for _, p in ipairs(protos) do
        local result = shell_exec("protoc --rust_out=src/proto " .. p)
        if not result.success then
            return {
                stdout = "",
                stderr = "protoc failed: " .. result.stderr,
                success = false,
            }
        end
        count = count + 1
    end

    return {
        stdout = "generated " .. count .. " proto files",
        stderr = "",
        success = true,
    }
end
```

## Plugin management

```bash
must plugin list              # list discovered plugins
must plugin check protoc      # validate a plugin
must plugin install <URL>     # install from URL
must plugin remove protoc     # remove a plugin
```
