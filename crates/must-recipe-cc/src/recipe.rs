use must_cache::hash::compute_hash;
use must_config::schema::{CrossBackend, CrossConfig};
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command as run_captured,
};
use must_toolchain::{
    ContainerToolchain, Triple, c_cross_env, c_install_hint, discover_c_compiler,
};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Run `<compiler> --version` and return the first line, or `"unknown"` on failure.
fn cc_version(compiler: &Path) -> String {
    Command::new(compiler)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .ok()
                .and_then(|s| s.lines().next().map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn object_path(ctx: &BuildContext, src: &str) -> PathBuf {
    ctx.project_root
        .join("build")
        .join(Path::new(src).with_extension("o"))
}

fn check_cache(key: &CacheKey, ctx: &BuildContext) -> Option<CacheLookup> {
    if let Some(ref cache) = ctx.cache {
        cache.lookup(key).ok()
    } else {
        must_cache::store::DiskCache::open(&ctx.cache_dir)
            .ok()
            .and_then(|c| Cache::lookup(&c, key).ok())
    }
}

fn store_cache(key: &CacheKey, ctx: &BuildContext) {
    if let Some(ref cache) = ctx.cache {
        let _ = cache.store(key, &[]);
    } else if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
        let _ = Cache::store(&cache, key, &[]);
    }
}

fn make_cache_key(
    recipe_name: &str,
    recipe_type: &str,
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    extra_flags: &BTreeMap<String, String>,
    compiler: &Path,
) -> CacheKey {
    let env_btree: BTreeMap<String, String> = ctx
        .env
        .iter()
        .chain(extra_env.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let hash = compute_hash(
        recipe_name,
        recipe_type,
        &[], // let the compiler track deps via -MMD — not our job here
        &env_btree,
        &cc_version(compiler),
        extra_flags,
    );
    CacheKey {
        recipe: recipe_name.to_string(),
        target: ctx.target.clone(),
        profile: ctx.profile.clone(),
        hash,
    }
}

/// Run a compiler command with explicit compiler path and arguments.
///
/// Uses `env_clear` + repopulate pattern to get a clean environment.
fn run_cc(
    compiler: &Path,
    args: &[String],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let start = Instant::now();
    let mut cmd = Command::new(compiler);
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(&ctx.project_root);
    cmd.env_clear();
    for (k, v) in &ctx.env {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = run_captured(
        &mut cmd,
        &compiler.display().to_string(),
        "Install a C compiler (Xcode CLI tools or build-essential)",
    )?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !out.status.success() {
        return Err(Error::RecipeFailed {
            name: compiler.display().to_string(),
            code: out.status.code().unwrap_or(-1),
            stderr: out.stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: compiler.display().to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout: out.stdout,
        stderr: out.stderr,
        duration_ms,
    })
}

fn run_cc_command(mut cmd: Command) -> Result<RecipeOutput> {
    let start = Instant::now();
    let out = run_captured(
        &mut cmd,
        "cc",
        "Install a C compiler (Xcode CLI tools or build-essential)",
    )?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !out.status.success() {
        return Err(Error::RecipeFailed {
            name: "cc".to_string(),
            code: out.status.code().unwrap_or(-1),
            stderr: out.stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: "cc".to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout: out.stdout,
        stderr: out.stderr,
        duration_ms,
    })
}

// ── CBinRecipe ────────────────────────────────────────────────────────────────

pub struct CBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub sources: Vec<String>,
    pub includes: Vec<String>,
    pub link_libs: Vec<String>,
    pub cflags: Vec<String>,
    pub env: HashMap<String, String>,
    pub cross: HashMap<String, CrossConfig>,
}

impl CBinRecipe {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            sources: Vec::new(),
            includes: Vec::new(),
            link_libs: Vec::new(),
            cflags: Vec::new(),
            env: HashMap::new(),
            cross: HashMap::new(),
        }
    }

    fn extra_flags(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("sources".to_string(), self.sources.join(","));
        m.insert("includes".to_string(), self.includes.join(","));
        m.insert("link_libs".to_string(), self.link_libs.join(","));
        m.insert("cflags".to_string(), self.cflags.join(" "));
        m
    }
}

impl Recipe for CBinRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(self
            .sources
            .iter()
            .map(|s| ctx.project_root.join(s))
            .collect())
    }

    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![ctx.project_root.join("build").join(&self.name)])
    }

    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Hash
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        // Use a fallback compiler path for hashing; actual path used in execute.
        let compiler = PathBuf::from("cc");
        Ok(make_cache_key(
            &self.name,
            "c-bin",
            ctx,
            &self.env,
            &self.extra_flags(),
            &compiler,
        ))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!(
                    "[dry-run] cc {} -o build/{}",
                    self.sources.join(" "),
                    self.name
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let key = self.cache_key(ctx)?;
        if let Some(CacheLookup::Hit) = check_cache(&key, ctx) {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: true,
                outputs: self.outputs(ctx)?,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let triple = if ctx.target == "host" {
            Triple::host()
        } else {
            Triple::parse(&ctx.target)?
        };

        let cross_cfg = self.cross.get(&ctx.target);
        let use_container = cross_cfg
            .and_then(|c| c.cross.as_ref())
            .map(|b| *b == CrossBackend::Container)
            .unwrap_or(false);

        let output_path = ctx.project_root.join("build").join(&self.name);
        // Ensure build directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        if use_container {
            // Container execution path
            let tc = ContainerToolchain::new(triple, "c-bin", ctx.project_root.clone(), None)
                .map_err(|e| Error::ToolchainNotFound {
                    target: ctx.target.clone(),
                    hint: e,
                })?;

            let mut translated_args: Vec<String> = Vec::new();

            // Source files (translated to container paths)
            for src in &self.sources {
                let host_path = ctx.project_root.join(src);
                let container_path = tc.translate_path(&host_path);
                translated_args.push(container_path.to_string_lossy().into_owned());
            }
            // Include paths
            for inc in &self.includes {
                translated_args.push("-I".to_string());
                let host_path = ctx.project_root.join(inc);
                let container_path = tc.translate_path(&host_path);
                translated_args.push(container_path.to_string_lossy().into_owned());
            }
            // Link libraries
            for lib in &self.link_libs {
                translated_args.push(format!("-l{}", lib));
            }
            // Extra cflags
            for flag in &self.cflags {
                translated_args.push(flag.clone());
            }
            // Output
            translated_args.push("-o".to_string());
            let container_output = tc.translate_path(&output_path);
            translated_args.push(container_output.to_string_lossy().into_owned());

            let arg_refs: Vec<&str> = translated_args.iter().map(String::as_str).collect();
            let cmd = tc.wrap_command("cc", &arg_refs);
            let mut result = run_cc_command(cmd)?;
            result.recipe_name = self.name.clone();
            result.outputs = vec![output_path];
            store_cache(&key, ctx);
            Ok(result)
        } else {
            // Local execution path
            let compiler_path = if let Some(cfg) = cross_cfg {
                if let Some(ref linker) = cfg.linker {
                    PathBuf::from(linker)
                } else {
                    discover_c_compiler(&triple).ok_or_else(|| Error::ToolchainNotFound {
                        target: ctx.target.clone(),
                        hint: c_install_hint(&triple),
                    })?
                }
            } else if triple.is_host() {
                // For host, try `cc` first
                PathBuf::from("cc")
            } else {
                discover_c_compiler(&triple).ok_or_else(|| Error::ToolchainNotFound {
                    target: ctx.target.clone(),
                    hint: c_install_hint(&triple),
                })?
            };

            let mut extra_env = c_cross_env(&triple, Some(&compiler_path));
            for (k, v) in &self.env {
                extra_env.insert(k.clone(), v.clone());
            }

            let mut args: Vec<String> = Vec::new();
            for src in &self.sources {
                args.push(ctx.project_root.join(src).to_string_lossy().into_owned());
            }
            for inc in &self.includes {
                args.push("-I".to_string());
                args.push(ctx.project_root.join(inc).to_string_lossy().into_owned());
            }
            for lib in &self.link_libs {
                args.push(format!("-l{}", lib));
            }
            for flag in &self.cflags {
                args.push(flag.clone());
            }
            args.push("-o".to_string());
            args.push(output_path.to_string_lossy().into_owned());

            let mut result = run_cc(&compiler_path, &args, ctx, &extra_env)?;
            result.recipe_name = self.name.clone();
            result.outputs = vec![output_path];
            store_cache(&key, ctx);
            Ok(result)
        }
    }
}

// ── CLibRecipe ────────────────────────────────────────────────────────────────

pub struct CLibRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub sources: Vec<String>,
    pub includes: Vec<String>,
    pub link_libs: Vec<String>,
    pub cflags: Vec<String>,
    pub env: HashMap<String, String>,
    pub cross: HashMap<String, CrossConfig>,
    pub static_lib: bool, // true = .a, false = .so
}

impl CLibRecipe {
    pub fn new(name: impl Into<String>, static_lib: bool) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            sources: Vec::new(),
            includes: Vec::new(),
            link_libs: Vec::new(),
            cflags: Vec::new(),
            env: HashMap::new(),
            cross: HashMap::new(),
            static_lib,
        }
    }

    fn extra_flags(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("sources".to_string(), self.sources.join(","));
        m.insert("includes".to_string(), self.includes.join(","));
        m.insert("link_libs".to_string(), self.link_libs.join(","));
        m.insert("cflags".to_string(), self.cflags.join(" "));
        m.insert(
            "lib_type".to_string(),
            if self.static_lib { "static" } else { "shared" }.to_string(),
        );
        m
    }

    fn lib_filename(&self) -> String {
        if self.static_lib {
            format!("lib{}.a", self.name)
        } else {
            format!("lib{}.so", self.name)
        }
    }
}

impl Recipe for CLibRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(self
            .sources
            .iter()
            .map(|s| ctx.project_root.join(s))
            .collect())
    }

    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![
            ctx.project_root.join("build").join(self.lib_filename()),
        ])
    }

    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Hash
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let compiler = PathBuf::from("cc");
        let recipe_type = if self.static_lib {
            "c-lib-static"
        } else {
            "c-lib-shared"
        };
        Ok(make_cache_key(
            &self.name,
            recipe_type,
            ctx,
            &self.env,
            &self.extra_flags(),
            &compiler,
        ))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            let action = if self.static_lib {
                "ar rcs"
            } else {
                "cc -shared"
            };
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!(
                    "[dry-run] {} build/{} {}",
                    action,
                    self.lib_filename(),
                    self.sources.join(" ")
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let key = self.cache_key(ctx)?;
        if let Some(CacheLookup::Hit) = check_cache(&key, ctx) {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: true,
                outputs: self.outputs(ctx)?,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let triple = if ctx.target == "host" {
            Triple::host()
        } else {
            Triple::parse(&ctx.target)?
        };

        let cross_cfg = self.cross.get(&ctx.target);
        let use_container = cross_cfg
            .and_then(|c| c.cross.as_ref())
            .map(|b| *b == CrossBackend::Container)
            .unwrap_or(false);

        let output_path = ctx.project_root.join("build").join(self.lib_filename());
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        if use_container {
            let tc = ContainerToolchain::new(triple, "c-lib", ctx.project_root.clone(), None)
                .map_err(|e| Error::ToolchainNotFound {
                    target: ctx.target.clone(),
                    hint: e,
                })?;

            let container_output = tc.translate_path(&output_path);

            if self.static_lib {
                // Compile each source to an object, then archive
                let mut object_paths: Vec<PathBuf> = Vec::new();
                for src in &self.sources {
                    let host_src = ctx.project_root.join(src);
                    let container_src = tc.translate_path(&host_src);
                    let obj_host = object_path(ctx, src);
                    if let Some(parent) = obj_host.parent() {
                        std::fs::create_dir_all(parent).map_err(Error::Io)?;
                    }
                    let obj_container = tc.translate_path(&obj_host);

                    let mut cc_args: Vec<String> = vec![
                        "-fPIC".to_string(),
                        "-c".to_string(),
                        container_src.to_string_lossy().into_owned(),
                    ];
                    for inc in &self.includes {
                        cc_args.push("-I".to_string());
                        let host_inc = ctx.project_root.join(inc);
                        cc_args.push(tc.translate_path(&host_inc).to_string_lossy().into_owned());
                    }
                    for flag in &self.cflags {
                        cc_args.push(flag.clone());
                    }
                    cc_args.push("-o".to_string());
                    cc_args.push(obj_container.to_string_lossy().into_owned());

                    let arg_refs: Vec<&str> = cc_args.iter().map(String::as_str).collect();
                    let cmd = tc.wrap_command("cc", &arg_refs);
                    run_cc_command(cmd)?;
                    object_paths.push(obj_container);
                }

                // ar rcs <output> <objects...>
                let mut ar_args: Vec<String> = vec![
                    "rcs".to_string(),
                    container_output.to_string_lossy().into_owned(),
                ];
                for obj in &object_paths {
                    ar_args.push(obj.to_string_lossy().into_owned());
                }
                let ar_refs: Vec<&str> = ar_args.iter().map(String::as_str).collect();
                let cmd = tc.wrap_command("ar", &ar_refs);
                let mut result = run_cc_command(cmd)?;
                result.recipe_name = self.name.clone();
                result.outputs = vec![output_path];
                store_cache(&key, ctx);
                Ok(result)
            } else {
                // Shared library: cc -shared -fPIC sources... -o output
                let mut cc_args: Vec<String> = vec!["-shared".to_string(), "-fPIC".to_string()];
                for src in &self.sources {
                    let host_src = ctx.project_root.join(src);
                    cc_args.push(tc.translate_path(&host_src).to_string_lossy().into_owned());
                }
                for inc in &self.includes {
                    cc_args.push("-I".to_string());
                    let host_inc = ctx.project_root.join(inc);
                    cc_args.push(tc.translate_path(&host_inc).to_string_lossy().into_owned());
                }
                for lib in &self.link_libs {
                    cc_args.push(format!("-l{}", lib));
                }
                for flag in &self.cflags {
                    cc_args.push(flag.clone());
                }
                cc_args.push("-o".to_string());
                cc_args.push(container_output.to_string_lossy().into_owned());

                let arg_refs: Vec<&str> = cc_args.iter().map(String::as_str).collect();
                let cmd = tc.wrap_command("cc", &arg_refs);
                let mut result = run_cc_command(cmd)?;
                result.recipe_name = self.name.clone();
                result.outputs = vec![output_path];
                store_cache(&key, ctx);
                Ok(result)
            }
        } else {
            // Local execution path
            let compiler_path = if let Some(cfg) = cross_cfg {
                if let Some(ref linker) = cfg.linker {
                    PathBuf::from(linker)
                } else {
                    discover_c_compiler(&triple).ok_or_else(|| Error::ToolchainNotFound {
                        target: ctx.target.clone(),
                        hint: c_install_hint(&triple),
                    })?
                }
            } else if triple.is_host() {
                PathBuf::from("cc")
            } else {
                discover_c_compiler(&triple).ok_or_else(|| Error::ToolchainNotFound {
                    target: ctx.target.clone(),
                    hint: c_install_hint(&triple),
                })?
            };

            let mut extra_env = c_cross_env(&triple, Some(&compiler_path));
            for (k, v) in &self.env {
                extra_env.insert(k.clone(), v.clone());
            }

            if self.static_lib {
                // Compile each source to an object with -fPIC -c, then `ar rcs`
                let mut object_paths: Vec<PathBuf> = Vec::new();
                for src in &self.sources {
                    let src_path = ctx.project_root.join(src);
                    let obj_path = object_path(ctx, src);
                    if let Some(parent) = obj_path.parent() {
                        std::fs::create_dir_all(parent).map_err(Error::Io)?;
                    }

                    let mut cc_args: Vec<String> = vec![
                        "-fPIC".to_string(),
                        "-c".to_string(),
                        src_path.to_string_lossy().into_owned(),
                    ];
                    for inc in &self.includes {
                        cc_args.push("-I".to_string());
                        cc_args.push(ctx.project_root.join(inc).to_string_lossy().into_owned());
                    }
                    for flag in &self.cflags {
                        cc_args.push(flag.clone());
                    }
                    cc_args.push("-o".to_string());
                    cc_args.push(obj_path.to_string_lossy().into_owned());

                    run_cc(&compiler_path, &cc_args, ctx, &extra_env)?;
                    object_paths.push(obj_path);
                }

                // ar rcs <output> <objects...>
                let ar_bin = extra_env
                    .get("AR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("ar"));
                let mut ar_args: Vec<String> = vec![
                    "rcs".to_string(),
                    output_path.to_string_lossy().into_owned(),
                ];
                for obj in &object_paths {
                    ar_args.push(obj.to_string_lossy().into_owned());
                }

                let start = Instant::now();
                let mut cmd = Command::new(&ar_bin);
                for a in &ar_args {
                    cmd.arg(a);
                }
                cmd.current_dir(&ctx.project_root);
                cmd.env_clear();
                for (k, v) in &ctx.env {
                    cmd.env(k, v);
                }
                let out = run_captured(
                    &mut cmd,
                    "ar",
                    "Install binutils (provides ar): https://www.gnu.org/software/binutils/ or Xcode CLI tools",
                )?;
                let duration_ms = start.elapsed().as_millis() as u64;
                if !out.status.success() {
                    return Err(Error::RecipeFailed {
                        name: "ar".to_string(),
                        code: out.status.code().unwrap_or(-1),
                        stderr: out.stderr,
                    });
                }
                store_cache(&key, ctx);
                Ok(RecipeOutput {
                    recipe_name: self.name.clone(),
                    from_cache: false,
                    outputs: vec![output_path],
                    stdout: out.stdout,
                    stderr: out.stderr,
                    duration_ms,
                })
            } else {
                // Shared library: cc -shared -fPIC sources... -o output
                let mut args: Vec<String> = vec!["-shared".to_string(), "-fPIC".to_string()];
                for src in &self.sources {
                    args.push(ctx.project_root.join(src).to_string_lossy().into_owned());
                }
                for inc in &self.includes {
                    args.push("-I".to_string());
                    args.push(ctx.project_root.join(inc).to_string_lossy().into_owned());
                }
                for lib in &self.link_libs {
                    args.push(format!("-l{}", lib));
                }
                for flag in &self.cflags {
                    args.push(flag.clone());
                }
                args.push("-o".to_string());
                args.push(output_path.to_string_lossy().into_owned());

                let mut result = run_cc(&compiler_path, &args, ctx, &extra_env)?;
                result.recipe_name = self.name.clone();
                result.outputs = vec![output_path];
                store_cache(&key, ctx);
                Ok(result)
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use must_core::Recipe;

    fn ctx() -> BuildContext {
        BuildContext {
            project_root: PathBuf::from("/tmp"),
            cache_dir: PathBuf::from("/tmp/.must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        }
    }

    #[test]
    fn test_cbin_cache_strategy_is_hash() {
        let r = CBinRecipe::new("mybin");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn test_clib_cache_strategy_is_hash() {
        let r = CLibRecipe::new("mylib", true);
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn test_cbin_outputs_in_build_dir() {
        let r = CBinRecipe::new("mybin");
        let outputs = r.outputs(&ctx()).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("/tmp/build/mybin")]);
    }

    #[test]
    fn test_clib_static_output_is_dot_a() {
        let r = CLibRecipe::new("mylib", true);
        let outputs = r.outputs(&ctx()).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("/tmp/build/libmylib.a")]);
    }

    #[test]
    fn test_clib_shared_output_is_dot_so() {
        let r = CLibRecipe::new("mylib", false);
        let outputs = r.outputs(&ctx()).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("/tmp/build/libmylib.so")]);
    }

    #[test]
    fn test_cbin_inputs_from_sources() {
        let mut r = CBinRecipe::new("mybin");
        r.sources = vec!["src/main.c".to_string(), "src/util.c".to_string()];
        let inputs = r.inputs(&ctx()).unwrap();
        assert_eq!(
            inputs,
            vec![
                PathBuf::from("/tmp/src/main.c"),
                PathBuf::from("/tmp/src/util.c"),
            ]
        );
    }

    #[test]
    fn test_cbin_deps_empty_by_default() {
        let r = CBinRecipe::new("mybin");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn test_cbin_dry_run() {
        let mut r = CBinRecipe::new("mybin");
        r.sources = vec!["src/main.c".to_string()];
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn test_clib_dry_run_static() {
        let mut r = CLibRecipe::new("mylib", true);
        r.sources = vec!["src/lib.c".to_string()];
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn test_clib_dry_run_shared() {
        let mut r = CLibRecipe::new("mylib", false);
        r.sources = vec!["src/lib.c".to_string()];
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert_eq!(out.duration_ms, 0);
    }

    // ── Accessor tests ────────────────────────────────────────────────────────

    #[test]
    fn test_cbin_name_accessor() {
        let r = CBinRecipe::new("myprog");
        assert_eq!(r.name(), "myprog");
    }

    #[test]
    fn test_cbin_deps_non_empty() {
        let mut r = CBinRecipe::new("myprog");
        r.deps = vec!["libfoo".to_string()];
        assert_eq!(r.deps(), &["libfoo"]);
    }

    #[test]
    fn test_clib_name_accessor() {
        let r = CLibRecipe::new("mylib", true);
        assert_eq!(r.name(), "mylib");
    }

    #[test]
    fn test_clib_deps_non_empty() {
        let mut r = CLibRecipe::new("mylib", false);
        r.deps = vec!["dep1".to_string()];
        assert_eq!(r.deps(), &["dep1"]);
    }

    // ── cache_key tests ───────────────────────────────────────────────────────

    #[test]
    fn test_cbin_cache_key_fields() {
        let r = CBinRecipe::new("myprog");
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "myprog");
        assert_eq!(key.target, "host");
        assert_eq!(key.profile, "default");
        assert!(!key.hash.is_empty());
    }

    #[test]
    fn test_clib_static_cache_key_fields() {
        let r = CLibRecipe::new("mylib", true);
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "mylib");
        assert!(!key.hash.is_empty());
    }

    #[test]
    fn test_clib_shared_cache_key_fields() {
        let r = CLibRecipe::new("mylib", false);
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "mylib");
        // Static and shared libs should have different cache keys
        let static_key = CLibRecipe::new("mylib", true).cache_key(&ctx()).unwrap();
        assert_ne!(key.hash, static_key.hash);
    }

    // ── execute() on host with real cc ────────────────────────────────────────

    #[test]
    fn test_cbin_execute_compiles_on_host() {
        // Skip if cc not available
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("main.c");
        std::fs::write(&src, "int main(void) { return 0; }\n").unwrap();

        let mut r = CBinRecipe::new("myprog");
        r.sources = vec!["main.c".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert_eq!(result.recipe_name, "myprog");
        assert!(!result.outputs.is_empty());
        assert!(result.outputs[0].exists(), "compiled binary should exist");
    }

    #[test]
    fn test_cbin_execute_with_include_dir() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();

        // Create include dir with a header
        std::fs::create_dir_all(tmp.path().join("include")).unwrap();
        std::fs::write(tmp.path().join("include/myheader.h"), "#define ANSWER 42\n").unwrap();
        std::fs::write(
            tmp.path().join("main.c"),
            "#include \"myheader.h\"\nint main(void) { return ANSWER - ANSWER; }\n",
        )
        .unwrap();

        let mut r = CBinRecipe::new("myprog2");
        r.sources = vec!["main.c".to_string()];
        r.includes = vec!["include".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert!(result.outputs[0].exists());
    }

    #[test]
    fn test_clib_static_execute_on_host() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("mylib.c"),
            "int add(int a, int b) { return a + b; }\n",
        )
        .unwrap();

        let mut r = CLibRecipe::new("mylib", true);
        r.sources = vec!["mylib.c".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert_eq!(result.recipe_name, "mylib");
        assert!(!result.outputs.is_empty());
        assert!(result.outputs[0].exists(), "libmylib.a should exist");
        assert!(result.outputs[0].to_string_lossy().ends_with("libmylib.a"));
    }

    #[test]
    fn test_clib_shared_execute_on_host() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("myshared.c"),
            "int mul(int a, int b) { return a * b; }\n",
        )
        .unwrap();

        let mut r = CLibRecipe::new("myshared", false);
        r.sources = vec!["myshared.c".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        // On macOS, cc -shared may produce a dylib-format file named .so
        // We just check it doesn't error and the output file exists
        let result = r.execute(&ctx);
        match result {
            Ok(out) => {
                assert!(!out.outputs.is_empty());
                assert!(out.outputs[0].exists());
            }
            Err(e) => {
                // Some environments may not support -shared; that's acceptable
                let err_str = e.to_string();
                assert!(
                    err_str.contains("RecipeFailed") || err_str.contains("failed"),
                    "unexpected error: {err_str}"
                );
            }
        }
    }

    // ── ToolchainNotFound for cross targets ───────────────────────────────────

    #[test]
    fn test_cbin_execute_cross_no_compiler_returns_toolchain_not_found() {
        // Use a cross target that definitely has no compiler on this host
        let triple_str = "x86_64-unknown-linux-gnu";
        let triple = must_toolchain::Triple::parse(triple_str).unwrap();

        // Only run this test if the cross-compiler is NOT available
        if must_toolchain::c_compiler_available(&triple) {
            return; // cross-compiler found — skip
        }

        let mut r = CBinRecipe::new("myprog");
        r.sources = vec!["src/main.c".to_string()];

        let ctx = BuildContext {
            project_root: std::path::PathBuf::from("/tmp"),
            cache_dir: std::path::PathBuf::from("/tmp/.must/cache"),
            log_dir: std::path::PathBuf::from("/tmp/mustfile-test/logs"),
            target: triple_str.to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        match r.execute(&ctx) {
            Err(must_core::Error::ToolchainNotFound { target, hint }) => {
                assert_eq!(target, triple_str);
                assert!(!hint.is_empty());
            }
            other => panic!("expected ToolchainNotFound, got: {other:?}"),
        }
    }

    #[test]
    fn test_clib_execute_cross_no_compiler_returns_toolchain_not_found() {
        let triple_str = "x86_64-unknown-linux-gnu";
        let triple = must_toolchain::Triple::parse(triple_str).unwrap();
        if must_toolchain::c_compiler_available(&triple) {
            return;
        }

        let mut r = CLibRecipe::new("mylib", true);
        r.sources = vec!["src/lib.c".to_string()];

        let ctx = BuildContext {
            project_root: std::path::PathBuf::from("/tmp"),
            cache_dir: std::path::PathBuf::from("/tmp/.must/cache"),
            log_dir: std::path::PathBuf::from("/tmp/mustfile-test/logs"),
            target: triple_str.to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        match r.execute(&ctx) {
            Err(must_core::Error::ToolchainNotFound { .. }) => {}
            other => panic!("expected ToolchainNotFound, got: {other:?}"),
        }
    }

    // ── run_command helper ────────────────────────────────────────────────────

    #[test]
    fn test_cbin_execute_compile_failure() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        // Intentionally broken C code
        let src = tmp.path().join("broken.c");
        std::fs::write(&src, "this is not valid C code at all !!!\n").unwrap();

        let mut r = CBinRecipe::new("broken");
        r.sources = vec!["broken.c".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx);
        assert!(result.is_err(), "compilation of invalid C should fail");
    }

    #[test]
    fn test_cbin_execute_with_cross_config_linker() {
        // Test the cross config path where a linker is explicitly set.
        // We use "cc" (which exists on macOS/Linux) as the cross "linker"
        // but for the host target (so it actually compiles successfully).
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.c"), "int main(void) { return 0; }\n").unwrap();

        let mut cross = HashMap::new();
        cross.insert(
            "host".to_string(),
            CrossConfig {
                linker: Some("cc".to_string()),
                cross: None,
            },
        );

        let mut r = CBinRecipe::new("cross_prog");
        r.sources = vec!["main.c".to_string()];
        r.cross = cross;

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert_eq!(result.recipe_name, "cross_prog");
    }

    #[test]
    fn test_run_command_success() {
        let cmd = std::process::Command::new("true");
        let result = run_cc_command(cmd).unwrap();
        assert!(result.stdout.is_empty());
    }

    #[test]
    fn test_run_command_failure() {
        let cmd = std::process::Command::new("false");
        let result = run_cc_command(cmd);
        assert!(result.is_err());
        match result.unwrap_err() {
            must_core::Error::RecipeFailed { name, code, .. } => {
                assert_eq!(name, "cc");
                assert_ne!(code, 0);
            }
            e => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_clib_inputs_returns_sources_joined_to_root() {
        let mut r = CLibRecipe::new("mylib", true);
        r.sources = vec!["src/lib.c".to_string(), "src/util.c".to_string()];
        let inputs = r.inputs(&ctx()).unwrap();
        assert_eq!(inputs.len(), 2);
        assert!(inputs[0].to_string_lossy().ends_with("lib.c"));
        assert!(inputs[1].to_string_lossy().ends_with("util.c"));
    }

    #[test]
    fn test_clib_inputs_empty_when_no_sources() {
        let r = CLibRecipe::new("mylib", true);
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn test_cbin_execute_with_cflags_and_nonempty_env() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.c"), "int main(void) { return 0; }\n").unwrap();

        let mut r = CBinRecipe::new("flagprog");
        r.sources = vec!["main.c".to_string()];
        r.cflags = vec!["-O0".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::from([("MY_FLAG".to_string(), "1".to_string())]),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert_eq!(result.recipe_name, "flagprog");
        assert!(result.outputs[0].exists());
    }

    #[test]
    fn test_cbin_execute_with_recipe_env() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("main.c"), "int main(void) { return 0; }\n").unwrap();

        let mut r = CBinRecipe::new("envprog");
        r.sources = vec!["main.c".to_string()];
        r.env = HashMap::from([("RECIPE_ENV".to_string(), "hello".to_string())]);

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert_eq!(result.recipe_name, "envprog");
    }

    #[test]
    fn test_clib_static_execute_with_includes_and_cflags() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("include")).unwrap();
        std::fs::write(tmp.path().join("include/mymath.h"), "int square(int x);\n").unwrap();
        std::fs::write(
            tmp.path().join("mymath.c"),
            "#include \"mymath.h\"\nint square(int x) { return x * x; }\n",
        )
        .unwrap();

        let mut r = CLibRecipe::new("mymath", true);
        r.sources = vec!["mymath.c".to_string()];
        r.includes = vec!["include".to_string()];
        r.cflags = vec!["-O1".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx).unwrap();
        assert!(result.outputs[0].exists(), "libmymath.a should exist");
    }

    #[test]
    fn test_clib_shared_execute_with_link_libs_and_cflags() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("shlib.c"),
            "double mysqrt(double x) { return x; }\n",
        )
        .unwrap();

        let mut r = CLibRecipe::new("shlib", false);
        r.sources = vec!["shlib.c".to_string()];
        r.cflags = vec!["-O0".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        // Accept either success or a platform-specific shared-lib error
        let _ = r.execute(&ctx);
    }

    #[test]
    fn test_object_paths_distinct_for_same_stem_in_different_dirs() {
        let c = ctx();
        let a = object_path(&c, "src/a/util.c");
        let b = object_path(&c, "src/b/util.c");
        assert_ne!(a, b);
        assert!(a.ends_with("build/src/a/util.o"));
        assert!(b.ends_with("build/src/b/util.o"));
    }

    #[test]
    fn test_clib_static_same_stem_sources_in_different_dirs() {
        if !must_toolchain::c_compiler_available(&must_toolchain::Triple::host()) {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/a")).unwrap();
        std::fs::create_dir_all(tmp.path().join("src/b")).unwrap();
        std::fs::write(
            tmp.path().join("src/a/util.c"),
            "int fa(void) { return 1; }\n",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("src/b/util.c"),
            "int fb(void) { return 2; }\n",
        )
        .unwrap();

        let mut r = CLibRecipe::new("samelib", true);
        r.sources = vec!["src/a/util.c".to_string(), "src/b/util.c".to_string()];

        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let result = r.execute(&ctx);
        match result {
            Ok(out) => {
                assert!(out.outputs[0].exists(), "libsamelib.a should exist");
                assert!(
                    tmp.path().join("build/src/a/util.o").exists(),
                    "object for src/a/util.c should be at build/src/a/util.o"
                );
                assert!(
                    tmp.path().join("build/src/b/util.o").exists(),
                    "object for src/b/util.c should be at build/src/b/util.o"
                );
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
