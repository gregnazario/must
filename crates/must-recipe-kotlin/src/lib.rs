use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_gradle(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    workdir: &std::path::Path,
) -> Result<RecipeOutput> {
    let start = Instant::now();
    let mut cmd = Command::new("./gradlew");
    for arg in args {
        cmd.arg(arg);
    }
    cmd.current_dir(workdir);
    cmd.env_clear();
    for (k, v) in &ctx.env {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = run_command(
        &mut cmd,
        "./gradlew",
        "Install Gradle: https://gradle.org/install/ or add a Gradle wrapper to your project",
    )?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !out.status.success() {
        return Err(Error::RecipeFailed {
            name: "gradle".to_string(),
            code: out.status.code().unwrap_or(-1),
            stderr: out.stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: "gradle".to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout: out.stdout,
        stderr: out.stderr,
        duration_ms,
    })
}

fn make_cache_key(
    recipe_name: &str,
    recipe_type: &str,
    ctx: &BuildContext,
    extra_flags: &BTreeMap<String, String>,
) -> CacheKey {
    let env_btree: BTreeMap<String, String> = ctx
        .env
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let hash = compute_hash(recipe_name, recipe_type, &[], &env_btree, "", extra_flags);
    CacheKey {
        recipe: recipe_name.to_string(),
        target: ctx.target.clone(),
        profile: ctx.profile.clone(),
        hash,
    }
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

fn workdir_path(ctx: &BuildContext, workdir: &str) -> std::path::PathBuf {
    if workdir == "." {
        ctx.project_root.clone()
    } else {
        ctx.project_root.join(workdir)
    }
}

pub struct KotlinBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl KotlinBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for KotlinBinRecipe {
    fn name(&self) -> &str {
        &self.name
    }
    fn deps(&self) -> &[String] {
        &self.deps
    }
    fn inputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn outputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Hash
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let mut flags = BTreeMap::new();
        flags.insert("package".to_string(), self.package.clone());
        Ok(make_cache_key(&self.name, "kotlin-bin", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        let key = self.cache_key(ctx)?;
        if let Some(CacheLookup::Hit) = check_cache(&key, ctx) {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: true,
                outputs: Vec::new(),
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!(
                    "[dry-run] ./gradlew build (in {})",
                    self.package
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_gradle(&["build"], ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

pub struct KotlinTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl KotlinTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for KotlinTestRecipe {
    fn name(&self) -> &str {
        &self.name
    }
    fn deps(&self) -> &[String] {
        &self.deps
    }
    fn inputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn outputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Never
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let mut flags = BTreeMap::new();
        flags.insert("package".to_string(), self.package.clone());
        Ok(make_cache_key(&self.name, "kotlin-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] ./gradlew test (in {})", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_gradle(&["test"], ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        Ok(result)
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
    fn kotlin_bin_cache_strategy_is_hash() {
        let r = KotlinBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn kotlin_bin_name_and_package() {
        let r = KotlinBinRecipe::new("build", "services/api");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "services/api");
    }

    #[test]
    fn kotlin_bin_deps_empty_by_default() {
        let r = KotlinBinRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn kotlin_bin_inputs_outputs_empty() {
        let r = KotlinBinRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn kotlin_bin_dry_run() {
        let r = KotlinBinRecipe::new("build", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("gradlew build"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn kotlin_bin_cache_hit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };
        let r = KotlinBinRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out = r.execute(&ctx);
        match out {
            Ok(o) => {
                assert!(o.from_cache);
                assert_eq!(o.recipe_name, "build");
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn kotlin_bin_cache_key_differs_by_package() {
        let r1 = KotlinBinRecipe::new("r", "pkg-a");
        let r2 = KotlinBinRecipe::new("r", "pkg-b");
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn kotlin_test_cache_strategy_is_never() {
        let r = KotlinTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn kotlin_test_name_and_package() {
        let r = KotlinTestRecipe::new("test", "services/api");
        assert_eq!(r.name(), "test");
        assert_eq!(r.package, "services/api");
    }

    #[test]
    fn kotlin_test_dry_run() {
        let r = KotlinTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("gradlew test"));
    }

    #[test]
    fn kotlin_bin_workdir_not_dot_dry_run() {
        let r = KotlinBinRecipe::new("build", "libs/core");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("libs/core"));
    }

    #[test]
    fn kotlin_bin_tool_not_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };
        let r = KotlinBinRecipe::new("build", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn kotlin_test_inputs_outputs_empty() {
        let r = KotlinTestRecipe::new("test", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn kotlin_bin_cache_key_stable() {
        let r = KotlinBinRecipe::new("build", ".");
        let key1 = r.cache_key(&ctx()).unwrap();
        let key2 = r.cache_key(&ctx()).unwrap();
        assert_eq!(key1.hash, key2.hash);
    }

    #[test]
    fn kotlin_test_workdir_not_dot_dry_run() {
        let r = KotlinTestRecipe::new("test", "services/api");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("services/api"));
    }

    #[test]
    fn kotlin_test_deps_empty() {
        let r = KotlinTestRecipe::new("test", ".");
        assert!(r.deps().is_empty());
    }
}
