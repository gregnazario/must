function inputs(ctx)
    local files = glob(ctx.project_root .. "/proto/*.proto")
    return files
end

function outputs(ctx)
    return { "src/generated/proto.rs" }
end

function execute(ctx)
    local protos = glob(ctx.project_root .. "/proto/*.proto")
    local count = 0

    mkdir(ctx.project_root .. "/src/generated")

    for _, p in ipairs(protos) do
        local result = shell_exec("protoc --rust_out=src/generated " .. p)
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
