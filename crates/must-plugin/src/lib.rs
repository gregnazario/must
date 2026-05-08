pub mod stdlib;

use must_core::error::Result;
use must_core::traits::Recipe;
use must_core::types::{BuildContext, CacheKey, CacheStrategy, RecipeOutput};
use mlua::Lua;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::warn;

pub struct LuaRecipe {
    name: String,
    deps: Vec<String>,
    lua: Mutex<Lua>,
    script: String,
}

impl LuaRecipe {
    pub fn load(name: &str, path: &Path) -> Result<Self> {
        let lua = Lua::new();
        stdlib::inject(&lua).map_err(|e| must_core::Error::Config {
            path: path.to_path_buf(),
            message: format!("failed to inject stdlib: {e}"),
        })?;

        let script = std::fs::read_to_string(path).map_err(must_core::Error::Io)?;

        lua.load(&script)
            .exec()
            .map_err(|e| must_core::Error::Config {
                path: path.to_path_buf(),
                message: format!("lua plugin error in {}: {e}", path.display()),
            })?;

        let globals = lua.globals();
        let has_execute: bool = globals
            .get::<mlua::Function>("execute")
            .map(|_| true)
            .unwrap_or(false);

        if !has_execute {
            return Err(must_core::Error::Config {
                path: path.to_path_buf(),
                message: format!(
                    "plugin '{}' must define an `execute(ctx)` function",
                    path.display()
                ),
            });
        }

        let deps: Vec<String> = globals
            .get::<Option<mlua::Table>>("deps")
            .ok()
            .flatten()
            .map(|t| {
                t.sequence_values::<String>()
                    .filter_map(|v| v.ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name: name.to_string(),
            deps,
            lua: Mutex::new(lua),
            script,
        })
    }

    fn with_lua<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&Lua) -> T,
    {
        let lua = self.lua.lock().expect("lua mutex poisoned");
        f(&lua)
    }

    fn call_string_fn(&self, fn_name: &str, ctx: &BuildContext) -> Option<String> {
        self.with_lua(|lua| {
            let globals = lua.globals();
            let func: mlua::Function = globals.get(fn_name).ok()?;
            let lua_ctx = build_lua_context(lua, ctx).ok()?;
            let result: String = func.call(lua_ctx).ok()?;
            Some(result)
        })
    }

    fn call_strings_fn(&self, fn_name: &str, ctx: &BuildContext) -> Option<Vec<String>> {
        self.with_lua(|lua| {
            let globals = lua.globals();
            let func: mlua::Function = globals.get(fn_name).ok()?;
            let lua_ctx = build_lua_context(lua, ctx).ok()?;
            let table: mlua::Table = func.call(lua_ctx).ok()?;
            Some(
                table
                    .sequence_values::<String>()
                    .filter_map(|v| v.ok())
                    .collect(),
            )
        })
    }
}

fn build_lua_context(lua: &Lua, ctx: &BuildContext) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("project_root", ctx.project_root.to_string_lossy().to_string())?;
    table.set("cache_dir", ctx.cache_dir.to_string_lossy().to_string())?;
    table.set("target", ctx.target.as_str())?;
    table.set("profile", ctx.profile.as_str())?;
    table.set("dry_run", ctx.dry_run)?;

    let env_table = lua.create_table()?;
    for (k, v) in &ctx.env {
        env_table.set(k.as_str(), v.as_str())?;
    }
    table.set("env", env_table)?;

    Ok(table)
}

impl Recipe for LuaRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(self
            .call_strings_fn("inputs", ctx)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect())
    }

    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(self
            .call_strings_fn("outputs", ctx)
            .unwrap_or_default()
            .into_iter()
            .map(PathBuf::from)
            .collect())
    }

    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Mtime
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let hash = self
            .call_string_fn("cache_key", ctx)
            .unwrap_or_else(|| self.script.clone());
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash: must_cache::hash::compute_hash(
                &self.name,
                "lua-plugin",
                &[],
                &std::collections::BTreeMap::new(),
                &hash,
                &std::collections::BTreeMap::new(),
            ),
        })
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        self.with_lua(|lua| {
            let globals = lua.globals();
            let func: mlua::Function = globals
                .get("execute")
                .map_err(|e| must_core::Error::Config {
                    path: PathBuf::from(&self.name),
                    message: format!("plugin execute() not found: {e}"),
                })?;

            let lua_ctx = build_lua_context(lua, ctx).map_err(|e| {
                must_core::Error::Config {
                    path: PathBuf::from(&self.name),
                    message: format!("lua context build error: {e}"),
                }
            })?;

            let table: mlua::Table = func.call(lua_ctx).map_err(|e| {
                must_core::Error::RecipeFailed {
                    name: self.name.clone(),
                    code: 1,
                    stderr: format!("lua execute() error: {e}"),
                }
            })?;

            let stdout: String = table.get("stdout").unwrap_or_default();
            let stderr: String = table.get("stderr").unwrap_or_default();
            let success: bool = table.get("success").unwrap_or(true);

            if !success {
                return Err(must_core::Error::RecipeFailed {
                    name: self.name.clone(),
                    code: 1,
                    stderr: stderr.clone(),
                });
            }

            if !stdout.is_empty() {
                print!("{stdout}");
                if !stdout.ends_with('\n') {
                    println!();
                }
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
                if !stderr.ends_with('\n') {
                    eprintln!();
                }
            }

            Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout,
                stderr,
                duration_ms: 0,
            })
        })
    }
}

pub fn discover_plugins(plugin_dir: &Path) -> Vec<LuaRecipe> {
    if !plugin_dir.exists() {
        return vec![];
    }

    let mut plugins = Vec::new();
    if let Ok(entries) = std::fs::read_dir(plugin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "lua") {
                let name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match LuaRecipe::load(&name, &path) {
                    Ok(plugin) => plugins.push(plugin),
                    Err(e) => warn!("skipping plugin {}: {e}", path.display()),
                }
            }
        }
    }
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_ctx() -> BuildContext {
        BuildContext {
            project_root: PathBuf::from("/tmp/test"),
            cache_dir: PathBuf::from("/tmp/test/.mustfile/cache"),
            log_dir: PathBuf::from("/tmp/test/.mustfile/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        }
    }

    #[test]
    fn test_load_and_execute_simple_plugin() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("greet.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    return {
        stdout = "hello from " .. ctx.target,
        stderr = "",
        success = true,
    }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("greet", &plugin_path).unwrap();
        assert_eq!(recipe.name(), "greet");
        assert!(recipe.deps().is_empty());

        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout, "hello from host");
    }

    #[test]
    fn test_plugin_missing_execute_fails() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("bad.lua");
        std::fs::write(
            &plugin_path,
            r#"
function something_else(ctx)
    return {}
end
"#,
        )
        .unwrap();

        let result = LuaRecipe::load("bad", &plugin_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_with_deps() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("deps.lua");
        std::fs::write(
            &plugin_path,
            r#"
deps = {"codegen", "compile"}

function execute(ctx)
    return { stdout = "ok", stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("deps", &plugin_path).unwrap();
        assert_eq!(recipe.deps(), &["codegen", "compile"]);
    }

    #[test]
    fn test_plugin_execute_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("fail.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    return { stdout = "", stderr = "something went wrong", success = false }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("fail", &plugin_path).unwrap();
        let result = recipe.execute(&test_ctx());
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_env_access() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("envtest.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local mode = ctx.env["MODE"] or "unset"
    return { stdout = "mode=" .. mode, stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("envtest", &plugin_path).unwrap();
        let mut ctx = test_ctx();
        ctx.env.insert("MODE".to_string(), "release".to_string());
        let output = recipe.execute(&ctx).unwrap();
        assert_eq!(output.stdout, "mode=release");
    }

    #[test]
    fn test_plugin_inputs_outputs() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("io.lua");
        std::fs::write(
            &plugin_path,
            r#"
function inputs(ctx)
    return { "src/main.rs", "Cargo.toml" }
end

function outputs(ctx)
    return { "target/debug/myapp" }
end

function execute(ctx)
    return { stdout = "built", stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("io", &plugin_path).unwrap();
        let ctx = test_ctx();

        let inputs = recipe.inputs(&ctx).unwrap();
        assert_eq!(
            inputs,
            vec![PathBuf::from("src/main.rs"), PathBuf::from("Cargo.toml")]
        );

        let outputs = recipe.outputs(&ctx).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("target/debug/myapp")]);
    }

    #[test]
    fn test_discover_plugins() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        std::fs::write(
            plugin_dir.join("alpha.lua"),
            r#"
function execute(ctx) return { stdout = "a", stderr = "", success = true } end
"#,
        )
        .unwrap();
        std::fs::write(
            plugin_dir.join("beta.lua"),
            r#"
function execute(ctx) return { stdout = "b", stderr = "", success = true } end
"#,
        )
        .unwrap();
        std::fs::write(plugin_dir.join("broken.lua"), "this is not valid lua {{{{").unwrap();
        std::fs::write(plugin_dir.join("readme.txt"), "not a plugin").unwrap();

        let plugins = discover_plugins(&plugin_dir);
        assert_eq!(plugins.len(), 2);
        let names: Vec<&str> = plugins.iter().map(|p| p.name()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_discover_plugins_empty_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugins = discover_plugins(dir.path().join("nonexistent").as_path());
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_plugin_lua_error_in_execute() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("error.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    error("intentional error")
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("error", &plugin_path).unwrap();
        let result = recipe.execute(&test_ctx());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("intentional error"),
            "error should contain lua message, got: {msg}"
        );
    }

    #[test]
    fn test_stdlib_shell_exec() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("shell.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local result = shell_exec("echo hello")
    return { stdout = result.stdout, stderr = "", success = result.success }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("shell", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert!(output.stdout.contains("hello"), "got: {}", output.stdout);
    }

    #[test]
    fn test_stdlib_file_io() {
        let dir = tempfile::TempDir::new().unwrap();
        let test_file = dir.path().join("test.txt");
        let plugin_path = dir.path().join("fio.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    write_file("{path}", "hello world")
    local content = read_file("{path}")
    return {{ stdout = content, stderr = "", success = true }}
end
"#,
                path = test_file.display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("fio", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout, "hello world");
    }

    #[test]
    fn test_stdlib_file_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        let existing = dir.path().join("exists.txt");
        std::fs::write(&existing, "yes").unwrap();
        let plugin_path = dir.path().join("exists.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    local a = file_exists("{path}")
    local b = file_exists("/nonexistent/path")
    return {{ stdout = tostring(a) .. " " .. tostring(b), stderr = "", success = true }}
end
"#,
                path = existing.display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("exists", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "true false");
    }

    #[test]
    fn test_stdlib_mkdir() {
        let dir = tempfile::TempDir::new().unwrap();
        let new_dir = dir.path().join("sub/nested");
        let plugin_path = dir.path().join("mkdir.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    mkdir("{path}")
    return {{ stdout = tostring(file_exists("{path}")), stderr = "", success = true }}
end
"#,
                path = new_dir.display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("mkdir", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "true");
    }

    #[test]
    fn test_stdlib_glob() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        std::fs::write(dir.path().join("c.rs"), "").unwrap();
        let plugin_path = dir.path().join("glob.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    local files = glob("{base}/*.txt")
    local out = ""
    for _, f in ipairs(files) do
        out = out .. f .. ","
    end
    return {{ stdout = out, stderr = "", success = true }}
end
"#,
                base = dir.path().display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("glob", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert!(output.stdout.contains("a.txt"), "got: {}", output.stdout);
        assert!(output.stdout.contains("b.txt"), "got: {}", output.stdout);
        assert!(!output.stdout.contains("c.rs"), "should not match .rs");
    }

    #[test]
    fn test_stdlib_env_get() {
        unsafe { std::env::set_var("MUST_TEST_PLUGIN_VAR", "testval"); }
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("envget.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local val = env_get("MUST_TEST_PLUGIN_VAR") or "unset"
    return { stdout = "val=" .. val, stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("envget", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout, "val=testval");
    }

    #[test]
    fn test_stdlib_shell_exec_failure() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("fail.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local result = shell_exec("exit 42")
    return {
        stdout = tostring(result.success) .. " code=" .. tostring(result.exit_code),
        stderr = result.stderr,
        success = true,
    }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("fail", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert!(output.stdout.contains("false"), "got: {}", output.stdout);
        assert!(output.stdout.contains("code=42"), "got: {}", output.stdout);
    }

    #[test]
    fn test_stdlib_glob_no_matches() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("glob_empty.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    local files = glob("{base}/*.nonexistent")
    return {{ stdout = tostring(#files), stderr = "", success = true }}
end
"#,
                base = dir.path().display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("glob_empty", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "0");
    }

    #[test]
    fn test_stdlib_env_get_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("env_missing.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local val = env_get("MUST_DEFINITELY_NOT_SET_XYZ123")
    local result = val == nil and "nil" or val
    return { stdout = result, stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("env_missing", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "nil");
    }

    #[test]
    fn test_stdlib_set_env() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("setenv.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    set_env("MUST_SET_TEST", "hello")
    local val = env_get("MUST_SET_TEST") or "missing"
    return { stdout = val, stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("setenv", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "hello");
    }

    #[test]
    fn test_stdlib_log_info() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("loginfo.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    log_info("test info message")
    return { stdout = "ok", stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("loginfo", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "ok");
    }

    #[test]
    fn test_stdlib_log_warn() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("logwarn.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    log_warn("test warning message")
    return { stdout = "ok", stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("logwarn", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "ok");
    }

    #[test]
    fn test_stdlib_read_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("readmissing.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    local ok, err = pcall(read_file, "/nonexistent/path/file.txt")
    if ok then
        return { stdout = "unexpected success", stderr = "", success = true }
    else
        return { stdout = "error", stderr = err, success = true }
    end
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("readmissing", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "error");
    }

    #[test]
    fn test_stdlib_write_and_read_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("output.txt");
        let plugin_path = dir.path().join("roundtrip.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    write_file("{path}", "hello world")
    local content = read_file("{path}")
    return {{ stdout = content, stderr = "", success = true }}
end
"#,
                path = target.display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("roundtrip", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "hello world");
    }

    #[test]
    fn test_stdlib_mkdir_nested() {
        let dir = tempfile::TempDir::new().unwrap();
        let nested = dir.path().join("a/b/c");
        let plugin_path = dir.path().join("mkdir_nested.lua");
        std::fs::write(
            &plugin_path,
            format!(
                r#"
function execute(ctx)
    mkdir("{path}")
    return {{ stdout = tostring(file_exists("{path}")), stderr = "", success = true }}
end
"#,
                path = nested.display()
            ),
        )
        .unwrap();

        let recipe = LuaRecipe::load("mkdir_nested", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout.trim(), "true");
    }

    #[test]
    fn test_plugin_cache_key_custom() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("custom_key.lua");
        std::fs::write(
            &plugin_path,
            r#"
function cache_key(ctx)
    return "custom-" .. ctx.profile
end
function execute(ctx)
    return { stdout = "ok", stderr = "", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("custom_key", &plugin_path).unwrap();
        let key = recipe.cache_key(&test_ctx()).unwrap();
        assert_eq!(key.recipe, "custom_key");
        assert_eq!(key.target, "host");
        assert_eq!(key.profile, "default");
        assert!(!key.hash.is_empty());
    }

    #[test]
    fn test_plugin_execute_stdout_stderr_printed() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("output.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    return { stdout = "hello out\n", stderr = "hello err\n", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("output", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout, "hello out\n");
        assert_eq!(output.stderr, "hello err\n");
    }

    #[test]
    fn test_plugin_execute_no_newline_printed() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("no_newline.lua");
        std::fs::write(
            &plugin_path,
            r#"
function execute(ctx)
    return { stdout = "no newline", stderr = "err no newline", success = true }
end
"#,
        )
        .unwrap();

        let recipe = LuaRecipe::load("no_newline", &plugin_path).unwrap();
        let output = recipe.execute(&test_ctx()).unwrap();
        assert_eq!(output.stdout, "no newline");
        assert_eq!(output.stderr, "err no newline");
    }

    #[test]
    fn test_discover_plugins_skips_invalid() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugin_dir).unwrap();

        std::fs::write(plugin_dir.join("good.lua"), r#"
function execute(ctx) return { stdout = "ok", stderr = "", success = true } end
"#).unwrap();
        std::fs::write(plugin_dir.join("bad.lua"), "invalid lua {{{{").unwrap();
        std::fs::write(plugin_dir.join("notes.txt"), "not a plugin").unwrap();

        let plugins = discover_plugins(&plugin_dir);
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name(), "good");
    }

    #[test]
    fn test_plugin_load_nonexistent() {
        let result = LuaRecipe::load("missing", std::path::Path::new("/nonexistent/plugin.lua"));
        assert!(result.is_err());
    }

    #[test]
    fn test_plugin_load_bad_lua_syntax() {
        let dir = tempfile::TempDir::new().unwrap();
        let plugin_path = dir.path().join("bad_syntax.lua");
        std::fs::write(&plugin_path, "function execute( invalid {{{{").unwrap();
        let result = LuaRecipe::load("bad_syntax", &plugin_path);
        assert!(result.is_err());
    }
}
