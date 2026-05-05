use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_status,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_cmd(
    program: &str,
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let start = Instant::now();
    let mut cmd = Command::new(program);
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
    let status = run_status(cmd.status(), program, "Install Node.js: https://nodejs.org")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !status.success() {
        return Err(Error::RecipeFailed {
            name: program.to_string(),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }
    Ok(RecipeOutput {
        recipe_name: program.to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout: String::new(),
        stderr: String::new(),
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
    must_cache::store::DiskCache::open(&ctx.cache_dir)
        .ok()
        .and_then(|c| c.lookup(key).ok())
}

fn store_cache(key: &CacheKey, ctx: &BuildContext) {
    if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
        let _ = cache.store(key, &[]);
    }
}

// ── TsBinRecipe ──────────────────────────────────────────────────────────────

pub struct TsBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl TsBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }

    fn extra_flags(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("package".to_string(), self.package.clone());
        m
    }
}

impl Recipe for TsBinRecipe {
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
        Ok(make_cache_key(
            &self.name,
            "ts-bin",
            ctx,
            &self.extra_flags(),
        ))
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
                stdout: format!("[dry-run] tsc --project {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let args = vec!["--project", &self.package];
        let mut result = run_cmd("tsc", &args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

// ── TsCheckRecipe ────────────────────────────────────────────────────────────

pub struct TsCheckRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl TsCheckRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for TsCheckRecipe {
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
        Ok(make_cache_key(&self.name, "ts-check", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] tsc --noEmit --project {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["--noEmit", "--project", &self.package];
        let mut result = run_cmd("tsc", &args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

// ── TsLintRecipe ─────────────────────────────────────────────────────────────

pub struct TsLintRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl TsLintRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for TsLintRecipe {
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
        Ok(make_cache_key(&self.name, "ts-lint", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] biome check {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["check", &self.package];
        let mut result = run_cmd("biome", &args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

// ── NpmRecipe ────────────────────────────────────────────────────────────────

pub struct NpmRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub npm_script: String,
    pub workdir: String,
    pub env: HashMap<String, String>,
}

impl NpmRecipe {
    pub fn new(name: impl Into<String>, npm_script: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            npm_script: npm_script.into(),
            workdir: ".".to_string(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for NpmRecipe {
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
        flags.insert("npm_script".to_string(), self.npm_script.clone());
        flags.insert("workdir".to_string(), self.workdir.clone());
        Ok(make_cache_key(&self.name, "npm", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!(
                    "[dry-run] npm run {} (in {})",
                    self.npm_script, self.workdir
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let npm_script = &self.npm_script;
        let args = vec!["run", npm_script];
        let mut result = run_cmd_in("npm", &args, ctx, &self.env, &self.workdir)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

fn run_cmd_in(
    program: &str,
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    workdir: &str,
) -> Result<RecipeOutput> {
    let start = Instant::now();
    let mut cmd = Command::new(program);
    for arg in args {
        cmd.arg(arg);
    }
    let work_dir = if workdir == "." {
        ctx.project_root.clone()
    } else {
        ctx.project_root.join(workdir)
    };
    cmd.current_dir(&work_dir);
    cmd.env_clear();
    for (k, v) in &ctx.env {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let status = run_status(cmd.status(), program, "Install Node.js: https://nodejs.org")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !status.success() {
        return Err(Error::RecipeFailed {
            name: program.to_string(),
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        });
    }
    Ok(RecipeOutput {
        recipe_name: program.to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout: String::new(),
        stderr: String::new(),
        duration_ms,
    })
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

    // ── TsBinRecipe ──────────────────────────────────────────────────────────

    #[test]
    fn ts_bin_cache_strategy_is_hash() {
        let r = TsBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn ts_bin_name_and_package() {
        let r = TsBinRecipe::new("build", "packages/api");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "packages/api");
    }

    #[test]
    fn ts_bin_deps_empty_by_default() {
        let r = TsBinRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn ts_bin_inputs_always_empty() {
        let r = TsBinRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn ts_bin_outputs_always_empty() {
        let r = TsBinRecipe::new("build", ".");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn ts_bin_dry_run_returns_without_spawning() {
        let r = TsBinRecipe::new("build", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("tsc"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn ts_bin_execute_returns_from_cache_when_hit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        };
        let r = TsBinRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);

        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache, "should return from cache");
        assert_eq!(out.recipe_name, "build");
    }

    // ── TsCheckRecipe ────────────────────────────────────────────────────────

    #[test]
    fn ts_check_cache_strategy_is_never() {
        let r = TsCheckRecipe::new("typecheck", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn ts_check_name_and_package() {
        let r = TsCheckRecipe::new("typecheck", "src");
        assert_eq!(r.name(), "typecheck");
        assert_eq!(r.package, "src");
    }

    #[test]
    fn ts_check_outputs_always_empty() {
        let r = TsCheckRecipe::new("typecheck", ".");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn ts_check_dry_run_returns_without_spawning() {
        let r = TsCheckRecipe::new("typecheck", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("--noEmit"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn ts_check_cache_key_includes_package() {
        let r1 = TsCheckRecipe::new("check", "pkg-a");
        let r2 = TsCheckRecipe::new("check", "pkg-b");
        let key1 = r1.cache_key(&ctx()).unwrap();
        let key2 = r2.cache_key(&ctx()).unwrap();
        assert_ne!(key1.hash, key2.hash, "package should affect cache key hash");
    }

    // ── TsLintRecipe ─────────────────────────────────────────────────────────

    #[test]
    fn ts_lint_cache_strategy_is_never() {
        let r = TsLintRecipe::new("lint", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn ts_lint_name_and_package() {
        let r = TsLintRecipe::new("lint", "src");
        assert_eq!(r.name(), "lint");
        assert_eq!(r.package, "src");
    }

    #[test]
    fn ts_lint_outputs_always_empty() {
        let r = TsLintRecipe::new("lint", ".");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn ts_lint_dry_run_returns_without_spawning() {
        let r = TsLintRecipe::new("lint", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("biome check"));
        assert_eq!(out.duration_ms, 0);
    }

    // ── NpmRecipe ───────────────────────────────────────────────────────────

    #[test]
    fn npm_cache_strategy_is_never() {
        let r = NpmRecipe::new("build", "build");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn npm_name_and_script() {
        let r = NpmRecipe::new("test", "test");
        assert_eq!(r.name(), "test");
        assert_eq!(r.npm_script, "test");
    }

    #[test]
    fn npm_default_workdir_is_dot() {
        let r = NpmRecipe::new("build", "build");
        assert_eq!(r.workdir, ".");
    }

    #[test]
    fn npm_deps_empty_by_default() {
        let r = NpmRecipe::new("build", "build");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn npm_outputs_always_empty() {
        let r = NpmRecipe::new("build", "build");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn npm_dry_run_returns_without_spawning() {
        let r = NpmRecipe::new("build", "build");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("npm run build"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn npm_dry_run_shows_workdir() {
        let mut r = NpmRecipe::new("build-api", "build");
        r.workdir = "packages/api".to_string();
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("packages/api"));
    }

    #[test]
    fn npm_cache_key_differs_by_script() {
        let r1 = NpmRecipe::new("r", "build");
        let r2 = NpmRecipe::new("r", "test");
        let key1 = r1.cache_key(&ctx()).unwrap();
        let key2 = r2.cache_key(&ctx()).unwrap();
        assert_ne!(key1.hash, key2.hash, "script name should affect cache key");
    }

    #[test]
    fn npm_cache_key_differs_by_workdir() {
        let mut r1 = NpmRecipe::new("r", "build");
        let mut r2 = NpmRecipe::new("r", "build");
        r1.workdir = "packages/a".to_string();
        r2.workdir = "packages/b".to_string();
        let key1 = r1.cache_key(&ctx()).unwrap();
        let key2 = r2.cache_key(&ctx()).unwrap();
        assert_ne!(key1.hash, key2.hash, "workdir should affect cache key");
    }
}
