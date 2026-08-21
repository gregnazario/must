use mlua::Lua;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type SharedWorkdir = Arc<Mutex<PathBuf>>;
pub type SharedEnv = Arc<Mutex<HashMap<String, String>>>;

pub fn inject(lua: &Lua, workdir: SharedWorkdir, plugin_env: SharedEnv) -> mlua::Result<()> {
    let globals = lua.globals();

    let shell_workdir = Arc::clone(&workdir);
    let shell_env = Arc::clone(&plugin_env);

    let shell_exec_fn = lua.create_function(move |lua_inner, cmd: String| {
        let mut command = must_core::shell_command(&cmd);
        if let Ok(dir) = shell_workdir.lock()
            && dir.is_dir()
        {
            command.current_dir(&*dir);
        }
        if let Ok(env) = shell_env.lock() {
            for (k, v) in env.iter() {
                command.env(k, v);
            }
        }
        let output = command.output();
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let table = lua_inner.create_table()?;
                table.set("success", out.status.success())?;
                table.set("exit_code", out.status.code().unwrap_or(-1))?;
                table.set("stdout", stdout)?;
                table.set("stderr", stderr)?;
                Ok(table)
            }
            Err(e) => Err(mlua::Error::external(format!(
                "failed to execute command: {e}"
            ))),
        }
    })?;
    globals.set("shell_exec", shell_exec_fn)?;

    let read_file_fn = lua.create_function(|_, path: String| {
        std::fs::read_to_string(&path)
            .map_err(|e| mlua::Error::external(format!("failed to read {path}: {e}")))
    })?;
    globals.set("read_file", read_file_fn)?;

    let write_file_fn = lua.create_function(|_, (path, content): (String, String)| {
        std::fs::write(&path, &content)
            .map_err(|e| mlua::Error::external(format!("failed to write {path}: {e}")))
    })?;
    globals.set("write_file", write_file_fn)?;

    let file_exists_fn =
        lua.create_function(|_, path: String| Ok(std::path::Path::new(&path).exists()))?;
    globals.set("file_exists", file_exists_fn)?;

    let mkdir_fn = lua.create_function(|_, path: String| {
        std::fs::create_dir_all(&path)
            .map_err(|e| mlua::Error::external(format!("failed to mkdir {path}: {e}")))
    })?;
    globals.set("mkdir", mkdir_fn)?;

    let glob_fn = lua.create_function(|lua_inner, pattern: String| {
        let table = lua_inner.create_table()?;
        if let Ok(entries) = glob::glob(&pattern) {
            for (i, entry) in entries.flatten().enumerate() {
                table.set(i + 1, entry.to_string_lossy().to_string())?;
            }
        }
        Ok(table)
    })?;
    globals.set("glob", glob_fn)?;

    let env_get_env = Arc::clone(&plugin_env);
    let env_get_fn = lua.create_function(move |_, key: String| {
        if let Ok(env) = env_get_env.lock()
            && let Some(v) = env.get(&key)
        {
            return Ok(Some(v.clone()));
        }
        Ok(std::env::var(&key).ok())
    })?;
    globals.set("env_get", env_get_fn)?;

    let set_env_target = Arc::clone(&plugin_env);
    let set_env_fn = lua.create_function(move |_, (key, value): (String, String)| {
        if let Ok(mut env) = set_env_target.lock() {
            env.insert(key, value);
        }
        Ok(())
    })?;
    globals.set("set_env", set_env_fn)?;

    let log_info_fn = lua.create_function(|_, msg: String| {
        tracing::info!("[plugin] {msg}");
        Ok(())
    })?;
    globals.set("log_info", log_info_fn)?;

    let log_warn_fn = lua.create_function(|_, msg: String| {
        tracing::warn!("[plugin] {msg}");
        Ok(())
    })?;
    globals.set("log_warn", log_warn_fn)?;

    Ok(())
}
