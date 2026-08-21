use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn nim_version() -> String {
    Command::new("nim")
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

fn run_nim(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("nim");
    let start = Instant::now();
    let mut cmd = Command::new("nim");
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
    let out = run_command(
        &mut cmd,
        "nim",
        "Install Nim: https://nim-lang.org/install.html",
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
    extra_env: &HashMap<String, String>,
    extra_flags: &BTreeMap<String, String>,
) -> CacheKey {
    let env_btree: BTreeMap<String, String> = ctx
        .env
        .iter()
        .chain(extra_env.iter())
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
        let _ = cache.store(key, &ctx.project_root, &[]);
    } else if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
        let _ = Cache::store(&cache, key, &ctx.project_root, &[]);
    }
}

pub struct NimBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl NimBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for NimBinRecipe {
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
        flags.insert("nim_version".to_string(), nim_version());
        Ok(make_cache_key(
            &self.name, "nim-bin", ctx, &self.env, &flags,
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
                stdout: format!("[dry-run] nim c -d:release {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["c", "-d:release", &self.package];
        let mut result = run_nim(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

pub struct NimTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl NimTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for NimTestRecipe {
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
        Ok(make_cache_key(
            &self.name, "nim-test", ctx, &self.env, &flags,
        ))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] nim r --hints:off {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["r", "--hints:off", &self.package];
        let mut result = run_nim(&args, ctx, &self.env)?;
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
    fn nim_bin_cache_strategy_is_hash() {
        let r = NimBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn nim_bin_name_and_package() {
        let r = NimBinRecipe::new("build", "src/main.nim");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "src/main.nim");
    }

    #[test]
    fn nim_bin_deps_empty() {
        let r = NimBinRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn nim_bin_inputs_outputs_empty() {
        let r = NimBinRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn nim_bin_dry_run() {
        let r = NimBinRecipe::new("build", "src/main.nim");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("nim c -d:release"));
        assert!(out.stdout.contains("src/main.nim"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn nim_bin_cache_hit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };
        let r = NimBinRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, tmp.path(), &[]).unwrap();
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
    fn nim_bin_cache_key_differs_by_package() {
        let r1 = NimBinRecipe::new("r", "src/a.nim");
        let r2 = NimBinRecipe::new("r", "src/b.nim");
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn nim_bin_tool_not_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };
        let r = NimBinRecipe::new("build", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn nim_test_cache_strategy_is_never() {
        let r = NimTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn nim_test_name_and_package() {
        let r = NimTestRecipe::new("test", "tests/test_all.nim");
        assert_eq!(r.name(), "test");
        assert_eq!(r.package, "tests/test_all.nim");
    }

    #[test]
    fn nim_test_dry_run() {
        let r = NimTestRecipe::new("test", "tests/test_all.nim");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("nim r"));
        assert!(out.stdout.contains("tests/test_all.nim"));
    }

    #[test]
    fn nim_test_inputs_outputs_empty() {
        let r = NimTestRecipe::new("test", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn nim_bin_dry_run_dot_package() {
        let r = NimBinRecipe::new("build", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("nim c -d:release ."));
    }

    #[test]
    fn nim_bin_cache_key_stable() {
        let r = NimBinRecipe::new("build", ".");
        let key1 = r.cache_key(&ctx()).unwrap();
        let key2 = r.cache_key(&ctx()).unwrap();
        assert_eq!(key1.hash, key2.hash);
    }

    #[test]
    fn nim_test_deps_empty() {
        let r = NimTestRecipe::new("test", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn nim_test_tool_not_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ctx = BuildContext {
            project_root: tmp.path().to_owned(),
            cache_dir: tmp.path().join(".must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
            cache: None,
        };
        let r = NimTestRecipe::new("test", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn nim_bin_dry_run_with_named_package() {
        let r = NimBinRecipe::new("build", "src/myapp.nim");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("src/myapp.nim"));
        assert!(out.stdout.contains("-d:release"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn nim_test_dry_run_shows_hints_off() {
        let r = NimTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("--hints:off"));
    }

    #[cfg(unix)]
    fn shim_ctx(tmp: &tempfile::TempDir, record: &std::path::Path) -> BuildContext {
        use std::os::unix::fs::PermissionsExt;
        let bin = tmp.path().join("shimbin");
        std::fs::create_dir_all(&bin).unwrap();
        let shim = bin.join("nim");
        std::fs::write(
            &shim,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"{r}\"\ncat .mustcwd >> \"{r}\" 2>&1\n",
                r = record.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::write(tmp.path().join(".mustcwd"), "project-root\n").unwrap();
        let mut c = ctx();
        let path = std::env::var("PATH").unwrap_or_default();
        c.env
            .insert("PATH".to_string(), format!("{}:{}", bin.display(), path));
        c.env
            .insert("MUST_RECORD".to_string(), record.display().to_string());
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        c
    }

    #[test]
    #[cfg(unix)]
    fn nim_bin_runs_in_project_root_with_package_arg() {
        let tmp = tempfile::TempDir::new().unwrap();
        let record = tmp.path().join("record.txt");
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::write(tmp.path().join("src/main.nim"), "echo 1\n").unwrap();
        let c = shim_ctx(&tmp, &record);
        let r = NimBinRecipe::new("build", "src/main.nim");
        match r.execute(&c) {
            Ok(out) => {
                assert_eq!(out.recipe_name, "build");
                assert!(!out.from_cache);
                let recorded = std::fs::read_to_string(&record).unwrap();
                assert_eq!(recorded, "c\n-d:release\nsrc/main.nim\nproject-root\n");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn nim_test_runs_in_project_root_with_package_arg() {
        let tmp = tempfile::TempDir::new().unwrap();
        let record = tmp.path().join("record.txt");
        std::fs::create_dir_all(tmp.path().join("tests")).unwrap();
        std::fs::write(tmp.path().join("tests/test_all.nim"), "discard 1\n").unwrap();
        let c = shim_ctx(&tmp, &record);
        let r = NimTestRecipe::new("test", "tests/test_all.nim");
        match r.execute(&c) {
            Ok(out) => {
                assert_eq!(out.recipe_name, "test");
                assert!(!out.from_cache);
                let recorded = std::fs::read_to_string(&record).unwrap();
                assert_eq!(
                    recorded,
                    "r\n--hints:off\ntests/test_all.nim\nproject-root\n"
                );
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
