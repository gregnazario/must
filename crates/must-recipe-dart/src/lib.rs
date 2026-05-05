use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_dart(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    workdir: &std::path::Path,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("dart");
    let start = Instant::now();
    let mut cmd = Command::new("dart");
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
        "dart",
        "Install Dart SDK: https://dart.dev/get-dart",
    )?;
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

fn workdir_path(ctx: &BuildContext, workdir: &str) -> std::path::PathBuf {
    if workdir == "." {
        ctx.project_root.clone()
    } else {
        ctx.project_root.join(workdir)
    }
}

pub struct DartBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl DartBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DartBinRecipe {
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
        Ok(make_cache_key(&self.name, "dart-bin", ctx, &flags))
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
                    "[dry-run] dart compile exe {}",
                    self.package
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = ctx.project_root.clone();
        let args = vec!["compile", "exe", &self.package];
        let mut result = run_dart(&args, ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

pub struct DartTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl DartTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DartTestRecipe {
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
        Ok(make_cache_key(&self.name, "dart-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] dart test (in {})", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let args = vec!["test"];
        let mut result = run_dart(&args, ctx, &self.env, &dir)?;
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
        }
    }

    fn ctx_with_path() -> BuildContext {
        let mut env = HashMap::new();
        if let Ok(path) = std::env::var("PATH") {
            env.insert("PATH".to_string(), path);
        }
        if let Ok(home) = std::env::var("HOME") {
            env.insert("HOME".to_string(), home);
        }
        BuildContext {
            project_root: PathBuf::from("/tmp"),
            cache_dir: PathBuf::from("/tmp/.mustfile/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env,
            dry_run: false,
            parallelism: 1,
        }
    }

    #[test]
    fn dart_bin_cache_strategy_is_hash() {
        let r = DartBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn dart_bin_name_and_package() {
        let r = DartBinRecipe::new("build", "bin/myapp.dart");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "bin/myapp.dart");
    }

    #[test]
    fn dart_bin_deps_empty() {
        let r = DartBinRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn dart_bin_inputs_outputs_empty() {
        let r = DartBinRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn dart_bin_dry_run() {
        let r = DartBinRecipe::new("build", "bin/main.dart");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("dart compile exe"));
        assert!(out.stdout.contains("bin/main.dart"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn dart_bin_cache_hit() {
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
        };
        let r = DartBinRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache);
    }

    #[test]
    fn dart_bin_cache_key_differs_by_package() {
        let r1 = DartBinRecipe::new("r", "bin/a.dart");
        let r2 = DartBinRecipe::new("r", "bin/b.dart");
        assert_ne!(r1.cache_key(&ctx()).unwrap().hash, r2.cache_key(&ctx()).unwrap().hash);
    }

    #[test]
    fn dart_test_cache_strategy_is_never() {
        let r = DartTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn dart_test_name_and_package() {
        let r = DartTestRecipe::new("test", "test/");
        assert_eq!(r.name(), "test");
        assert_eq!(r.package, "test/");
    }

    #[test]
    fn dart_test_dry_run() {
        let r = DartTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("dart test"));
    }

    #[test]
    fn dart_test_workdir_in_dry_run() {
        let r = DartTestRecipe::new("test", "pkgs/api");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("pkgs/api"));
    }

    #[test]
    fn dart_bin_execute_real() {
        if std::process::Command::new("dart")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let bindir = tmp.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        std::fs::write(bindir.join("main.dart"), "void main() { print('hello'); }\n").unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".mustfile/cache");
        let mut r = DartBinRecipe::new("build", "bin/main.dart");
        r.package = "bin/main.dart".to_string();
        let result = r.execute(&c);
        match result {
            Ok(out) => {
                assert_eq!(out.recipe_name, "build");
                assert!(!out.from_cache);
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn dart_bin_tool_not_found() {
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
        };
        let r = DartBinRecipe::new("build", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn dart_test_inputs_outputs_empty() {
        let r = DartTestRecipe::new("test", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }
}
