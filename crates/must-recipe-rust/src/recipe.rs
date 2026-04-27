use crate::toolchain::rustc_version;
use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

// ── Shared helpers ─────────────────────────────────────────────────────────────

fn run_cargo(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("cargo");
    let start = Instant::now();
    let mut cmd = Command::new("cargo");
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
    let output = cmd.output().map_err(Error::Io)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let duration_ms = start.elapsed().as_millis() as u64;

    if !output.status.success() {
        return Err(Error::RecipeFailed {
            name: name.to_string(),
            code: output.status.code().unwrap_or(-1),
            stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: name.to_string(),
        from_cache: false,
        outputs: Vec::new(),
        stdout,
        stderr,
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
    let hash = compute_hash(
        recipe_name,
        recipe_type,
        &[], // no explicit input files — cargo tracks its own deps
        &env_btree,
        &rustc_version(),
        extra_flags,
    );
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

// ── RustBinRecipe ─────────────────────────────────────────────────────────────

pub struct RustBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub features: Vec<String>,
    pub release: bool,
    pub env: HashMap<String, String>,
}

impl RustBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            features: Vec::new(),
            release: false,
            env: HashMap::new(),
        }
    }

    fn extra_flags(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("package".to_string(), self.package.clone());
        m.insert("features".to_string(), self.features.join(","));
        m.insert("release".to_string(), self.release.to_string());
        m
    }
}

impl Recipe for RustBinRecipe {
    fn name(&self) -> &str {
        &self.name
    }
    fn deps(&self) -> &[String] {
        &self.deps
    }
    fn inputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }
    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        let profile = if self.release { "release" } else { "debug" };
        Ok(vec![ctx
            .project_root
            .join("target")
            .join(profile)
            .join(&self.package)])
    }
    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Hash
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        Ok(make_cache_key(
            &self.name,
            "rust-bin",
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
                outputs: self.outputs(ctx)?,
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
                stdout: format!("[dry-run] cargo build -p {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let mut args = vec!["build", "--package", &self.package];
        let feature_str;
        if !self.features.is_empty() {
            feature_str = self.features.join(",");
            args.extend_from_slice(&["--features", &feature_str]);
        }
        if self.release {
            args.push("--release");
        }

        let mut result = run_cargo(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        result.outputs = self.outputs(ctx)?;
        store_cache(&key, ctx);
        Ok(result)
    }
}

// ── RustLibRecipe ─────────────────────────────────────────────────────────────

pub struct RustLibRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub features: Vec<String>,
    pub release: bool,
    pub env: HashMap<String, String>,
}

impl RustLibRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            features: Vec::new(),
            release: false,
            env: HashMap::new(),
        }
    }
}

impl Recipe for RustLibRecipe {
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
        flags.insert("features".to_string(), self.features.join(","));
        Ok(make_cache_key(&self.name, "rust-lib", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        let key = self.cache_key(ctx)?;
        if let Some(CacheLookup::Hit) = check_cache(&key, ctx) {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: true,
                outputs: vec![],
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] cargo build --lib -p {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let mut args = vec!["build", "--lib", "--package", &self.package];
        let feature_str;
        if !self.features.is_empty() {
            feature_str = self.features.join(",");
            args.extend_from_slice(&["--features", &feature_str]);
        }
        if self.release {
            args.push("--release");
        }
        let mut result = run_cargo(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

// ── RustTestRecipe ────────────────────────────────────────────────────────────

pub struct RustTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub test_filter: Option<String>,
    pub env: HashMap<String, String>,
}

impl RustTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            test_filter: None,
            env: HashMap::new(),
        }
    }
}

impl Recipe for RustTestRecipe {
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
        if let Some(f) = &self.test_filter {
            flags.insert("filter".to_string(), f.clone());
        }
        Ok(make_cache_key(&self.name, "rust-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        // Tests are always re-run (no cache for test results — results can change with env)
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] cargo test -p {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let mut args = vec!["test", "--package", &self.package];
        if let Some(filter) = &self.test_filter {
            args.push(filter);
        }
        let mut result = run_cargo(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}
