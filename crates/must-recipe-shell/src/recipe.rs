use glob::glob;
use must_cache::mtime::check_mtime;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, CommandOutput, Error, Recipe,
    RecipeOutput, Result, run_command, shell_command, shell_display, shell_program,
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

    fn run(&self, ctx: &BuildContext) -> Result<CommandOutput> {
        let mut cmd = shell_command(&self.script);
        cmd.current_dir(&ctx.project_root);
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        run_command(&mut cmd, shell_program(), "A shell is required")
    }

    fn content_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        use must_cache::hash::compute_hash;
        use std::collections::BTreeMap;
        let inputs = self.inputs(ctx)?;
        let input_refs: Vec<&std::path::Path> = inputs.iter().map(|p| p.as_path()).collect();
        let env: BTreeMap<String, String> = ctx
            .env
            .iter()
            .chain(self.env.iter())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let hash = compute_hash(
            &self.name,
            "shell",
            &input_refs,
            &env,
            &self.script,
            &BTreeMap::new(),
        );
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash,
        })
    }

    fn cached_output(&self, outputs: Vec<PathBuf>) -> RecipeOutput {
        RecipeOutput {
            recipe_name: self.name.clone(),
            from_cache: true,
            outputs,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 0,
        }
    }

    fn check_cache(&self, ctx: &BuildContext) -> Result<CacheCheck> {
        match self.cache {
            CacheStrategy::Never => Ok(CacheCheck::Miss {
                key: None,
                owned_cache: None,
            }),
            CacheStrategy::Mtime => {
                let inputs = self.inputs(ctx)?;
                let outputs = self.outputs(ctx)?;
                let input_refs: Vec<&std::path::Path> =
                    inputs.iter().map(|p| p.as_path()).collect();
                let output_refs: Vec<&std::path::Path> =
                    outputs.iter().map(|p| p.as_path()).collect();
                if let CacheLookup::Hit = check_mtime(&input_refs, &output_refs)? {
                    Ok(CacheCheck::Hit(self.cached_output(outputs)))
                } else {
                    Ok(CacheCheck::Miss {
                        key: None,
                        owned_cache: None,
                    })
                }
            }
            CacheStrategy::Hash => {
                let key = self.content_key(ctx)?;
                let owned_cache = if ctx.cache.is_some() {
                    None
                } else {
                    let opened = must_cache::store::DiskCache::open(&ctx.cache_dir).ok();
                    if opened.is_none() {
                        eprintln!(
                            "warning: could not open cache at {} — rebuilding without cache",
                            ctx.cache_dir.display()
                        );
                    }
                    opened
                };
                let effective_cache: Option<&dyn Cache> = ctx
                    .cache
                    .as_deref()
                    .or_else(|| owned_cache.as_deref().map(|c| c as &dyn Cache));
                if let Some(cache) = effective_cache
                    && matches!(cache.lookup(&key), Ok(CacheLookup::Hit))
                {
                    let outputs = self.outputs(ctx)?;
                    let usable = self.outputs.is_empty()
                        || !outputs.is_empty()
                        || (cache.restore(&key, &ctx.project_root).unwrap_or(false)
                            && !self.outputs(ctx)?.is_empty());
                    if usable {
                        return Ok(CacheCheck::Hit(self.cached_output(self.outputs(ctx)?)));
                    }
                }
                Ok(CacheCheck::Miss {
                    key: Some(key),
                    owned_cache,
                })
            }
        }
    }
}

enum CacheCheck {
    Hit(RecipeOutput),
    Miss {
        key: Option<CacheKey>,
        owned_cache: Option<std::sync::Arc<must_cache::store::DiskCache>>,
    },
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
        if self.cache == CacheStrategy::Hash {
            return self.content_key(ctx);
        }
        let key_str = format!("{}:{}:{}", self.name, ctx.target, ctx.profile);
        let hash = must_cache::store::hash_string(&key_str);
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash,
        })
    }

    fn is_cached(&self, ctx: &BuildContext) -> Result<Option<bool>> {
        Ok(Some(matches!(self.check_cache(ctx)?, CacheCheck::Hit(_))))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if self.script.trim().is_empty() {
            return Err(Error::Config {
                path: ctx.project_root.clone(),
                message: format!("recipe '{}' has no script for this platform", self.name),
            });
        }
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

        let (key, owned_cache) = match self.check_cache(ctx)? {
            CacheCheck::Hit(output) => return Ok(output),
            CacheCheck::Miss { key, owned_cache } => (key, owned_cache),
        };
        let effective_cache: Option<&dyn Cache> = ctx
            .cache
            .as_deref()
            .or_else(|| owned_cache.as_deref().map(|c| c as &dyn Cache));

        let start = Instant::now();
        let out = self.run(ctx)?;
        let duration_ms = start.elapsed().as_millis() as u64;

        if !out.status.success() {
            return Err(Error::RecipeFailed {
                name: self.name.clone(),
                code: out.status.code().unwrap_or(-1),
                stderr: out.stderr,
            });
        }

        let outputs = self.outputs(ctx)?;

        if let (Some(cache), Some(key)) = (effective_cache, key)
            && let Err(e) = cache.store(&key, &ctx.project_root, &outputs)
        {
            eprintln!("warning: cache store failed: {e}");
        }

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
        let tmp = tempfile::TempDir::new().unwrap();
        let mut r = ShellRecipe::new(
            "greet-env",
            "echo \"$CTX_VAR|$RECIPE_VAR|$SHARED|${UNSET_VAR:-default}\"",
        );
        r.cache = CacheStrategy::Never;
        r.env = HashMap::from([
            ("RECIPE_VAR".to_string(), "recipe".to_string()),
            ("SHARED".to_string(), "recipe".to_string()),
        ]);

        let mut c = ctx();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        c.env = HashMap::from([
            ("CTX_VAR".to_string(), "ctx".to_string()),
            ("SHARED".to_string(), "ctx".to_string()),
        ]);

        let out = r.execute(&c).unwrap();
        assert_eq!(out.stdout.trim(), "ctx|recipe|recipe|default");
    }

    fn ctx_in(tmp: &tempfile::TempDir) -> BuildContext {
        BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: tmp.path().join(".must/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        }
    }

    #[test]
    fn is_cached_agrees_with_execute() {
        for strategy in [
            CacheStrategy::Mtime,
            CacheStrategy::Hash,
            CacheStrategy::Never,
        ] {
            let tmp = tempfile::TempDir::new().unwrap();
            std::fs::write(tmp.path().join("in.txt"), "v1").unwrap();
            let mut r = ShellRecipe::new("gen", "cat in.txt > out.txt");
            r.cache = strategy;
            r.inputs = vec!["in.txt".to_string()];
            r.outputs = vec!["out.txt".to_string()];
            let ctx = ctx_in(&tmp);

            assert_eq!(r.is_cached(&ctx).unwrap(), Some(false));
            assert!(!r.execute(&ctx).unwrap().from_cache);

            let cached_now = r.cache != CacheStrategy::Never;
            assert_eq!(r.is_cached(&ctx).unwrap(), Some(cached_now));
            assert_eq!(r.execute(&ctx).unwrap().from_cache, cached_now);

            std::fs::write(tmp.path().join("in.txt"), "v2").unwrap();
            assert_eq!(r.is_cached(&ctx).unwrap(), Some(false));
            assert!(!r.execute(&ctx).unwrap().from_cache);
        }
    }

    #[test]
    fn cache_key_is_the_key_execute_stores() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("in.txt"), "v1").unwrap();
        let mut r = ShellRecipe::new("gen", "cp in.txt out.txt");
        r.cache = CacheStrategy::Hash;
        r.inputs = vec!["in.txt".to_string()];
        r.outputs = vec!["out.txt".to_string()];
        let ctx = ctx_in(&tmp);
        r.execute(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        assert!(matches!(
            cache.lookup(&r.cache_key(&ctx).unwrap()).unwrap(),
            CacheLookup::Hit
        ));
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
