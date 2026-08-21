use glob::glob;
use must_cache::mtime::check_mtime;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command, shell_command, shell_display, shell_program,
};
use std::collections::HashMap;
use std::path::PathBuf;
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
                stdout: format!("[dry-run] would run: {}", shell_display(&self.script)),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

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
                &self.script,
                &BTreeMap::new(),
            );
            let key = CacheKey {
                recipe: self.name.clone(),
                target: ctx.target.clone(),
                profile: ctx.profile.clone(),
                hash,
            };

            let owned_cache;
            let effective_cache: Option<&dyn Cache> = if let Some(ref cache) = ctx.cache {
                Some(cache.as_ref())
            } else {
                owned_cache = must_cache::store::DiskCache::open(&ctx.cache_dir).ok();
                owned_cache.as_ref().map(|c| c as &dyn Cache)
            };
            if let Some(cache) = effective_cache
                && matches!(cache.lookup(&key), Ok(CacheLookup::Hit))
            {
                let outputs = self.outputs(ctx)?;
                let usable = self.outputs.is_empty()
                    || !outputs.is_empty()
                    || (cache.restore(&key, &ctx.project_root).unwrap_or(false)
                        && !self.outputs(ctx)?.is_empty());
                if usable {
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

            let start = Instant::now();
            let mut cmd = shell_command(&self.script);
            cmd.current_dir(&ctx.project_root);
            let out = run_command(&mut cmd, shell_program(), "A shell is required")?;
            let duration_ms = start.elapsed().as_millis() as u64;

            if !out.status.success() {
                return Err(Error::RecipeFailed {
                    name: self.name.clone(),
                    code: out.status.code().unwrap_or(-1),
                    stderr: out.stderr,
                });
            }

            let outputs = self.outputs(ctx)?;

            if let Some(cache) = effective_cache {
                let _ = cache.store(&key, &ctx.project_root, &outputs);
            }

            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs,
                stdout: out.stdout,
                stderr: out.stderr,
                duration_ms,
            });
        }

        let start = Instant::now();
        let mut cmd = shell_command(&self.script);
        cmd.current_dir(&ctx.project_root);
        let out = run_command(&mut cmd, shell_program(), "A shell is required")?;
        let duration_ms = start.elapsed().as_millis() as u64;

        if !out.status.success() {
            return Err(Error::RecipeFailed {
                name: self.name.clone(),
                code: out.status.code().unwrap_or(-1),
                stderr: out.stderr,
            });
        }

        let outputs = self.outputs(ctx)?;

        Ok(RecipeOutput {
            recipe_name: self.name.clone(),
            from_cache: false,
            outputs,
            stdout: out.stdout,
            stderr: out.stderr,
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
    fn execute_runs_script_and_returns_ok() {
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
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let out = r.execute(&c).unwrap();
        assert!(
            out.from_cache,
            "output exists with no newer inputs → should be a cache hit"
        );
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn execute_with_ctx_and_recipe_env() {
        // Cover the env loop bodies (ctx.env and self.env) in execute()
        let mut r = ShellRecipe::new("greet-env", "echo hello");
        r.env = HashMap::from([("RECIPE_VAR".to_string(), "world".to_string())]);

        let c = BuildContext {
            project_root: std::path::PathBuf::from("/tmp"),
            cache_dir: std::path::PathBuf::from("/tmp/.must/cache"),
            log_dir: std::path::PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::from([("CTX_VAR".to_string(), "ctx-value".to_string())]),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };

        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn hash_cache_stores_then_hits_on_second_run() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".must/cache")).unwrap();

        let mut r = ShellRecipe::new("codegen", "echo generated");
        r.cache = CacheStrategy::Hash;

        let c = BuildContext {
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

        let first = r.execute(&c).unwrap();
        assert!(!first.from_cache, "first run should not be from cache");

        let second = r.execute(&c).unwrap();
        assert!(
            second.from_cache,
            "second run with same inputs should be a cache hit"
        );
    }

    #[test]
    fn hash_cache_invalidated_when_script_changes() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".must/cache")).unwrap();
        std::fs::write(tmp.path().join("in.txt"), "input").unwrap();

        let mut r = ShellRecipe::new("gen", "echo one > out.txt");
        r.cache = CacheStrategy::Hash;
        r.inputs = vec!["in.txt".to_string()];
        r.outputs = vec!["out.txt".to_string()];

        let c = BuildContext {
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

        r.execute(&c).unwrap();
        let second = r.execute(&c).unwrap();
        assert!(second.from_cache, "same script should hit");

        r.script = "echo two > out.txt".to_string();
        let third = r.execute(&c).unwrap();
        assert!(
            !third.from_cache,
            "edited script must invalidate the hash cache"
        );
        let content = std::fs::read_to_string(tmp.path().join("out.txt")).unwrap();
        assert_eq!(content.trim(), "two");
    }

    #[test]
    fn hash_cache_restores_deleted_outputs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".must/cache")).unwrap();

        let mut r = ShellRecipe::new("gen2", "echo data > out.txt");
        r.cache = CacheStrategy::Hash;
        r.outputs = vec!["out.txt".to_string()];

        let c = BuildContext {
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

        r.execute(&c).unwrap();
        let second = r.execute(&c).unwrap();
        assert!(second.from_cache);

        std::fs::remove_file(tmp.path().join("out.txt")).unwrap();
        let third = r.execute(&c).unwrap();
        assert!(
            third.from_cache,
            "cache should restore the deleted output instead of rebuilding"
        );
        let content = std::fs::read_to_string(tmp.path().join("out.txt")).unwrap();
        assert_eq!(content.trim(), "data");
    }

    #[test]
    fn hash_cache_rebuilds_when_outputs_never_produced() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".must/cache")).unwrap();

        let mut r = ShellRecipe::new("gen3", "echo done");
        r.cache = CacheStrategy::Hash;
        r.outputs = vec!["never-created.txt".to_string()];

        let c = BuildContext {
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

        r.execute(&c).unwrap();
        let second = r.execute(&c).unwrap();
        assert!(
            !second.from_cache,
            "declared outputs that were never produced must trigger a rebuild"
        );
    }

    #[test]
    fn none_cache_strategy_always_runs() {
        let mut r = ShellRecipe::new("always", "echo run");
        r.cache = CacheStrategy::Never;
        let c = ctx();
        let first = r.execute(&c).unwrap();
        assert!(!first.from_cache);
        let second = r.execute(&c).unwrap();
        assert!(!second.from_cache);
    }

    #[test]
    fn mtime_cache_miss_when_no_outputs() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("in.txt"), "input").unwrap();

        let mut r = ShellRecipe::new("gen", "echo new-output");
        r.inputs = vec!["in.txt".to_string()];
        r.cache = CacheStrategy::Mtime;

        let c = BuildContext {
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

        let out = r.execute(&c).unwrap();
        assert!(!out.from_cache, "no outputs exist → should execute");
    }

    #[test]
    fn inputs_expand_glob_patterns() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "").unwrap();
        let mut r = ShellRecipe::new("build", "echo");
        r.inputs = vec!["*.txt".to_string()];
        let c = BuildContext {
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
        let inputs = r.inputs(&c).unwrap();
        assert_eq!(inputs.len(), 2);
    }

    #[test]
    fn outputs_expand_glob_patterns() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("out1.log"), "").unwrap();
        std::fs::write(tmp.path().join("out2.log"), "").unwrap();
        let mut r = ShellRecipe::new("build", "echo");
        r.outputs = vec!["*.log".to_string()];
        let c = BuildContext {
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
        let outputs = r.outputs(&c).unwrap();
        assert_eq!(outputs.len(), 2);
    }

    #[test]
    fn dry_run_contains_command_display() {
        let r = ShellRecipe::new("deploy", "scp file user@host:/path");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("scp"));
    }

    #[test]
    fn cache_key_differs_by_target() {
        let r = ShellRecipe::new("build", "echo");
        let mut c1 = ctx();
        c1.target = "linux".to_string();
        let mut c2 = ctx();
        c2.target = "macos".to_string();
        assert_ne!(
            r.cache_key(&c1).unwrap().hash,
            r.cache_key(&c2).unwrap().hash
        );
    }

    #[test]
    fn cache_key_differs_by_profile() {
        let r = ShellRecipe::new("build", "echo");
        let mut c1 = ctx();
        c1.profile = "debug".to_string();
        let mut c2 = ctx();
        c2.profile = "release".to_string();
        assert_ne!(
            r.cache_key(&c1).unwrap().hash,
            r.cache_key(&c2).unwrap().hash
        );
    }

    #[test]
    fn execute_runs_in_project_root() {
        let tmp = tempfile::TempDir::new().unwrap();
        let subdir = tmp.path().join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();

        let marker = tmp.path().join("marker.txt");
        std::fs::write(&marker, "from_root").unwrap();

        let r = ShellRecipe::new("check-cwd", "cat marker.txt");

        let c = BuildContext {
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

        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&subdir).unwrap();
        let result = r.execute(&c);
        std::env::set_current_dir(&orig).unwrap();

        match result {
            Ok(out) => {
                assert!(
                    out.stdout.contains("from_root"),
                    "shell should run in project_root, not cwd; stdout was: {}",
                    out.stdout.trim()
                );
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}
