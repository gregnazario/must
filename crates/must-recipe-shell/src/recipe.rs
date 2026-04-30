use glob::glob;
use must_cache::mtime::check_mtime;
use must_core::{
    BuildContext, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

pub struct ShellRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub inputs: Vec<String>,  // glob patterns
    pub outputs: Vec<String>, // glob patterns
    pub script: String,
    pub cache: CacheStrategy,
    pub env: HashMap<String, String>,
}

impl ShellRecipe {
    pub fn new(name: impl Into<String>, script: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            script: script.into(),
            cache: CacheStrategy::Mtime,
            env: HashMap::new(),
        }
    }
}

fn expand_globs(patterns: &[String], root: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let full_pattern = root.join(pattern).to_string_lossy().into_owned();
        for entry in glob(&full_pattern).map_err(|e| Error::Config {
            path: root.to_owned(),
            message: format!("invalid glob pattern '{pattern}': {e}"),
        })? {
            let path = entry.map_err(|e| Error::Io(e.into_error()))?;
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

impl Recipe for ShellRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        expand_globs(&self.inputs, &ctx.project_root)
    }

    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        expand_globs(&self.outputs, &ctx.project_root)
    }

    fn cache_strategy(&self) -> CacheStrategy {
        self.cache.clone()
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        // For mtime strategy, we just use a stable hash of the recipe identity.
        // For hash strategy, we'd hash the input contents too (M2 feature).
        let key_str = format!("{}:{}:{}", self.name, ctx.target, ctx.profile);
        let hash = must_cache::store::hash_string(&key_str);
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash,
        })
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!("[dry-run] would run: sh -c '{}'", self.script),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        // Check mtime cache for Mtime strategy
        if self.cache == CacheStrategy::Mtime {
            let inputs = self.inputs(ctx)?;
            let outputs = self.outputs(ctx)?;
            let input_refs: Vec<&std::path::Path> = inputs.iter().map(|p| p.as_path()).collect();
            let output_refs: Vec<&std::path::Path> = outputs.iter().map(|p| p.as_path()).collect();
            if let CacheLookup::Hit = check_mtime(&input_refs, &output_refs)? {
                return Ok(RecipeOutput {
                    recipe_name: self.name.clone(),
                    from_cache: true,
                    outputs,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration_ms: 0,
                });
            }
        }

        if self.cache == CacheStrategy::Hash {
            use must_cache::hash::compute_hash;
            use std::collections::BTreeMap;

            let inputs = self.inputs(ctx)?;
            let input_refs: Vec<&std::path::Path> = inputs.iter().map(|p| p.as_path()).collect();
            let env_btree: BTreeMap<String, String> = ctx
                .env
                .iter()
                .chain(self.env.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let hash = compute_hash(
                &self.name,
                "shell",
                &input_refs,
                &env_btree,
                "", // no toolchain for shell recipes
                &BTreeMap::new(),
            );
            // Check disk cache
            if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
                let key = CacheKey {
                    recipe: self.name.clone(),
                    target: ctx.target.clone(),
                    profile: ctx.profile.clone(),
                    hash,
                };
                use must_core::Cache;
                if let Ok(CacheLookup::Hit) = cache.lookup(&key) {
                    return Ok(RecipeOutput {
                        recipe_name: self.name.clone(),
                        from_cache: true,
                        outputs: self.outputs(ctx)?,
                        stdout: String::new(),
                        stderr: String::new(),
                        duration_ms: 0,
                    });
                }
            }
        }

        let start = Instant::now();
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&self.script);
        cmd.current_dir(&ctx.project_root);

        // Compose env: inherit from ctx, then recipe-level overrides
        cmd.env_clear();
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }

        let output = cmd.output().map_err(Error::Io)?;
        let duration_ms = start.elapsed().as_millis() as u64;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(Error::RecipeFailed {
                name: self.name.clone(),
                code: output.status.code().unwrap_or(-1),
                stderr: stderr.clone(),
            });
        }

        let outputs = self.outputs(ctx)?;

        // Store hash cache entry after a successful run
        if self.cache == CacheStrategy::Hash {
            use must_cache::hash::compute_hash;
            use std::collections::BTreeMap;

            let inputs = self.inputs(ctx)?;
            let input_refs: Vec<&std::path::Path> = inputs.iter().map(|p| p.as_path()).collect();
            let env_btree: BTreeMap<String, String> = ctx
                .env
                .iter()
                .chain(self.env.iter())
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            let hash = compute_hash(
                &self.name,
                "shell",
                &input_refs,
                &env_btree,
                "",
                &BTreeMap::new(),
            );
            if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
                let key = CacheKey {
                    recipe: self.name.clone(),
                    target: ctx.target.clone(),
                    profile: ctx.profile.clone(),
                    hash,
                };
                use must_core::Cache;
                let _ = cache.store(&key, &outputs);
            }
        }

        Ok(RecipeOutput {
            recipe_name: self.name.clone(),
            from_cache: false,
            outputs,
            stdout,
            stderr,
            duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use must_core::Recipe;
    use std::path::PathBuf;

    fn ctx() -> BuildContext {
        BuildContext {
            project_root: PathBuf::from("/tmp"),
            cache_dir: PathBuf::from("/tmp/.mustfile/cache"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        }
    }

    #[test]
    fn default_cache_strategy_is_mtime() {
        let r = ShellRecipe::new("build", "echo hi");
        assert_eq!(r.cache_strategy(), CacheStrategy::Mtime);
    }

    #[test]
    fn hash_cache_strategy_when_set() {
        let mut r = ShellRecipe::new("codegen", "echo gen");
        r.cache = CacheStrategy::Hash;
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn name_and_script_roundtrip() {
        let r = ShellRecipe::new("lint", "cargo clippy");
        assert_eq!(r.name(), "lint");
        assert_eq!(r.script, "cargo clippy");
    }

    #[test]
    fn deps_empty_by_default() {
        let r = ShellRecipe::new("build", "make");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn inputs_empty_when_no_globs() {
        let r = ShellRecipe::new("build", "echo");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn outputs_empty_when_no_patterns() {
        let r = ShellRecipe::new("build", "echo");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn dry_run_skips_execution() {
        let r = ShellRecipe::new("build", "exit 99");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert_eq!(out.duration_ms, 0);
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn execute_runs_script_and_captures_stdout() {
        let r = ShellRecipe::new("greet", "echo hello");
        let out = r.execute(&ctx()).unwrap();
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn execute_fails_on_nonzero_exit() {
        let r = ShellRecipe::new("bad", "exit 1");
        let result = r.execute(&ctx());
        assert!(result.is_err());
    }

    #[test]
    fn cache_key_returns_stable_hash() {
        let r = ShellRecipe::new("build", "echo hi");
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "build");
        assert_eq!(key.target, "host");
        assert_eq!(key.profile, "default");
        assert!(!key.hash.is_empty());
        // Calling twice should produce identical result
        let key2 = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.hash, key2.hash);
    }

    #[test]
    fn mtime_cache_hit_returns_from_cache() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create the output file before running the recipe
        std::fs::write(tmp.path().join("out.txt"), "cached result").unwrap();

        let mut r = ShellRecipe::new("gen", "echo should-not-run");
        r.outputs = vec!["out.txt".to_string()];
        // Default cache is Mtime

        let c = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        };

        let out = r.execute(&c).unwrap();
        assert!(out.from_cache, "output exists with no newer inputs → should be a cache hit");
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn execute_with_ctx_and_recipe_env() {
        // Cover the env loop bodies (ctx.env and self.env) in execute()
        let mut r = ShellRecipe::new("greet-env", "echo hello");
        r.env = HashMap::from([("RECIPE_VAR".to_string(), "world".to_string())]);

        let mut c = BuildContext {
            project_root: std::path::PathBuf::from("/tmp"),
            cache_dir: std::path::PathBuf::from("/tmp/.mustfile/cache"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::from([("CTX_VAR".to_string(), "ctx-value".to_string())]),
            dry_run: false,
            parallelism: 1,
        };

        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn hash_cache_stores_then_hits_on_second_run() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".mustfile/cache")).unwrap();

        let mut r = ShellRecipe::new("codegen", "echo generated");
        r.cache = CacheStrategy::Hash;

        let c = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        };

        let first = r.execute(&c).unwrap();
        assert!(!first.from_cache, "first run should not be from cache");

        let second = r.execute(&c).unwrap();
        assert!(second.from_cache, "second run with same inputs should be a cache hit");
    }
}
