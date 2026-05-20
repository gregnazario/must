function inputs(ctx)
    return glob(ctx.project_root .. "/assets/**/*")
end

function execute(ctx)
    local assets = glob(ctx.project_root .. "/assets/**/*")
    local count = 0

    write_file(ctx.project_root .. "/src/generated/assets.rs", "pub const ASSETS: &[(&str, &[u8])] = &[\n")

    for _, a in ipairs(assets) do
        local content = read_file(a)
        local name = a:gsub(ctx.project_root .. "/", "")
        write_file(
            ctx.project_root .. "/src/generated/assets.rs",
            '    ("' .. name .. '", b"' .. content .. '"),\n'
        )
        count = count + 1
    end

    write_file(ctx.project_root .. "/src/generated/assets.rs", "];\n")

    return {
        stdout = "embedded " .. count .. " assets",
        stderr = "",
        success = true,
    }
end
