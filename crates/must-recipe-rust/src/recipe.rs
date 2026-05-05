use crate::toolchain::rustc_version;
use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
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
    let out = run_command(&mut cmd, "cargo", "Install Rust: https://rustup.rs")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !out.status.success() {
        return Err(Error::RecipeFailed {
            name: name.to_string(),
            code: out.status.code().unwrap_or(-1),
            stderr: out.stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: name.to_string(),
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
        Ok(vec![
            ctx.project_root
                .join("target")
                .join(profile)
                .join(&self.package),
        ])
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

    // ── RustBinRecipe ─────────────────────────────────────────────────────────

    #[test]
    fn rust_bin_cache_strategy_is_hash() {
        let r = RustBinRecipe::new("build", "myapp");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn rust_bin_name_and_package() {
        let r = RustBinRecipe::new("build", "myapp");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "myapp");
    }

    #[test]
    fn rust_bin_deps_empty_by_default() {
        let r = RustBinRecipe::new("build", "myapp");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn rust_bin_inputs_always_empty() {
        let r = RustBinRecipe::new("build", "myapp");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn rust_bin_output_path_debug() {
        let r = RustBinRecipe::new("build", "myapp");
        let outputs = r.outputs(&ctx()).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("/tmp/target/debug/myapp")]);
    }

    #[test]
    fn rust_bin_output_path_release() {
        let mut r = RustBinRecipe::new("build", "myapp");
        r.release = true;
        let outputs = r.outputs(&ctx()).unwrap();
        assert_eq!(outputs, vec![PathBuf::from("/tmp/target/release/myapp")]);
    }

    // ── RustLibRecipe ─────────────────────────────────────────────────────────

    #[test]
    fn rust_lib_cache_strategy_is_hash() {
        let r = RustLibRecipe::new("lib", "mylib");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn rust_lib_inputs_empty() {
        let r = RustLibRecipe::new("lib", "mylib");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }

    // ── RustTestRecipe ────────────────────────────────────────────────────────

    #[test]
    fn rust_test_cache_strategy_is_never() {
        let r = RustTestRecipe::new("test", "myapp");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn rust_test_outputs_always_empty() {
        let r = RustTestRecipe::new("test", "myapp");
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn rust_test_dry_run_returns_without_spawning() {
        let r = RustTestRecipe::new("test", "myapp");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert_eq!(out.duration_ms, 0);
    }

    // ── Cache hit tests ───────────────────────────────────────────────────────

    #[test]
    fn rust_bin_execute_returns_from_cache_when_hit() {
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
        let r = RustBinRecipe::new("build", "myapp");
        // Pre-populate cache with the exact key the recipe would compute
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);

        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache, "should return from cache");
        assert_eq!(out.recipe_name, "build");
    }

    #[test]
    fn rust_bin_execute_dry_run_skips_cargo() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: true,
            parallelism: 1,
        };
        let r = RustBinRecipe::new("build", "myapp");
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(!out.from_cache);
    }

    #[test]
    fn rust_bin_execute_dry_run_with_features() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: true,
            parallelism: 1,
        };
        let mut r = RustBinRecipe::new("build", "myapp");
        r.features = vec!["feat-a".into()];
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn rust_bin_execute_dry_run_release() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: true,
            parallelism: 1,
        };
        let mut r = RustBinRecipe::new("build", "myapp");
        r.release = true;
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn rust_lib_execute_returns_from_cache_when_hit() {
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
        let r = RustLibRecipe::new("lib", "mylib");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);

        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache);
    }

    #[test]
    fn rust_lib_execute_dry_run() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: true,
            parallelism: 1,
        };
        let r = RustLibRecipe::new("lib", "mylib");
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(!out.from_cache);
    }

    #[test]
    fn rust_lib_execute_dry_run_with_features_and_release() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".mustfile/cache"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: true,
            parallelism: 1,
        };
        let mut r = RustLibRecipe::new("lib", "mylib");
        r.features = vec!["feat".into()];
        r.release = true;
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn rust_test_cache_key_includes_filter() {
        let mut r = RustTestRecipe::new("test", "myapp");
        r.test_filter = Some("my_specific_test".to_string());
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "test");
        // Verify that a key without filter differs
        let r2 = RustTestRecipe::new("test", "myapp");
        let key2 = r2.cache_key(&ctx()).unwrap();
        assert_ne!(key.hash, key2.hash, "filter should affect cache key hash");
    }

    #[test]
    fn rust_test_name_deps_inputs() {
        let mut r = RustTestRecipe::new("mytest", "mypkg");
        r.deps = vec!["build".to_string()];
        assert_eq!(r.name(), "mytest");
        assert_eq!(r.deps(), &["build"]);
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn rust_test_dry_run_with_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut r = RustTestRecipe::new("mytest", "mypkg");
        r.test_filter = Some("specific_test".to_string());
        let mut c = ctx();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".mustfile/cache");
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(!out.from_cache);
    }
}
