use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_dotnet(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("dotnet");
    let start = Instant::now();
    let mut cmd = Command::new("dotnet");
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
        "dotnet",
        "Install .NET SDK: https://dotnet.microsoft.com/download",
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

pub struct DotnetBuildRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl DotnetBuildRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DotnetBuildRecipe {
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
        Ok(make_cache_key(&self.name, "dotnet-build", ctx, &flags))
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
                stdout: format!("[dry-run] dotnet build {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["build", &self.package];
        let mut result = run_dotnet(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

pub struct DotnetTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl DotnetTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DotnetTestRecipe {
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
        Ok(make_cache_key(&self.name, "dotnet-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] dotnet test {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["test", &self.package];
        let mut result = run_dotnet(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

pub struct DotnetPublishRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl DotnetPublishRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DotnetPublishRecipe {
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
        Ok(make_cache_key(&self.name, "dotnet-publish", ctx, &flags))
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
                stdout: format!("[dry-run] dotnet publish {} -c Release", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let args = vec!["publish", &self.package, "-c", "Release"];
        let mut result = run_dotnet(&args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
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
            cache_dir: PathBuf::from("/tmp/.must/cache"),
            log_dir: PathBuf::from("/tmp/mustfile-test/logs"),
            target: "host".to_string(),
            profile: "default".to_string(),
            env,
            dry_run: false,
            parallelism: 1,
            cache: None,
        }
    }

    #[test]
    fn dotnet_build_cache_strategy_is_hash() {
        let r = DotnetBuildRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn dotnet_build_name_and_package() {
        let r = DotnetBuildRecipe::new("build", "src/MyApp");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "src/MyApp");
    }

    #[test]
    fn dotnet_build_deps_empty() {
        let r = DotnetBuildRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn dotnet_build_inputs_outputs_empty() {
        let r = DotnetBuildRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn dotnet_build_dry_run() {
        let r = DotnetBuildRecipe::new("build", "MyApp.csproj");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("dotnet build"));
        assert!(out.stdout.contains("MyApp.csproj"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn dotnet_build_cache_hit() {
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
        let r = DotnetBuildRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out = r.execute(&ctx);
        match out {
            Ok(o) => {
                assert!(o.from_cache);
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn dotnet_build_cache_key_differs_by_package() {
        let r1 = DotnetBuildRecipe::new("r", "AppA");
        let r2 = DotnetBuildRecipe::new("r", "AppB");
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn dotnet_test_cache_strategy_is_never() {
        let r = DotnetTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn dotnet_test_name_and_package() {
        let r = DotnetTestRecipe::new("test", "tests/MyApp.Tests");
        assert_eq!(r.name(), "test");
        assert_eq!(r.package, "tests/MyApp.Tests");
    }

    #[test]
    fn dotnet_test_dry_run() {
        let r = DotnetTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("dotnet test"));
    }

    #[test]
    fn dotnet_publish_cache_strategy_is_hash() {
        let r = DotnetPublishRecipe::new("publish", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn dotnet_publish_name_and_package() {
        let r = DotnetPublishRecipe::new("publish", "src/MyApp");
        assert_eq!(r.name(), "publish");
        assert_eq!(r.package, "src/MyApp");
    }

    #[test]
    fn dotnet_publish_dry_run() {
        let r = DotnetPublishRecipe::new("publish", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("dotnet publish"));
        assert!(out.stdout.contains("Release"));
    }

    #[test]
    fn dotnet_publish_cache_hit() {
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
        let r = DotnetPublishRecipe::new("publish", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out = r.execute(&ctx);
        match out {
            Ok(o) => {
                assert!(o.from_cache);
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn dotnet_build_execute_real() {
        if std::process::Command::new("dotnet")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = DotnetBuildRecipe::new("build", "MyApp.csproj");
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
    fn dotnet_build_tool_not_found() {
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
        let r = DotnetBuildRecipe::new("build", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn dotnet_test_inputs_outputs_empty() {
        let r = DotnetTestRecipe::new("test", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn dotnet_publish_inputs_outputs_empty() {
        let r = DotnetPublishRecipe::new("publish", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[cfg(unix)]
    fn shim_ctx(tmp: &tempfile::TempDir, record: &std::path::Path) -> BuildContext {
        use std::os::unix::fs::PermissionsExt;
        let bin = tmp.path().join("shimbin");
        std::fs::create_dir_all(&bin).unwrap();
        let shim = bin.join("dotnet");
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
        let mut c = ctx_with_path();
        let path = c.env.get("PATH").cloned().unwrap_or_default();
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
    fn dotnet_build_runs_in_project_root_with_package_arg() {
        let tmp = tempfile::TempDir::new().unwrap();
        let record = tmp.path().join("record.txt");
        std::fs::write(tmp.path().join("MyApp.csproj"), "").unwrap();
        let c = shim_ctx(&tmp, &record);
        let r = DotnetBuildRecipe::new("build", "MyApp.csproj");
        match r.execute(&c) {
            Ok(out) => {
                assert_eq!(out.recipe_name, "build");
                assert!(!out.from_cache);
                let recorded = std::fs::read_to_string(&record).unwrap();
                assert_eq!(recorded, "build\nMyApp.csproj\nproject-root\n");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn dotnet_test_runs_in_project_root_with_package_arg() {
        let tmp = tempfile::TempDir::new().unwrap();
        let record = tmp.path().join("record.txt");
        std::fs::write(tmp.path().join("MyApp.Tests.csproj"), "").unwrap();
        let c = shim_ctx(&tmp, &record);
        let r = DotnetTestRecipe::new("test", "MyApp.Tests.csproj");
        match r.execute(&c) {
            Ok(out) => {
                assert_eq!(out.recipe_name, "test");
                assert!(!out.from_cache);
                let recorded = std::fs::read_to_string(&record).unwrap();
                assert_eq!(recorded, "test\nMyApp.Tests.csproj\nproject-root\n");
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn dotnet_publish_runs_in_project_root_with_package_arg() {
        let tmp = tempfile::TempDir::new().unwrap();
        let record = tmp.path().join("record.txt");
        std::fs::write(tmp.path().join("MyApp.csproj"), "").unwrap();
        let c = shim_ctx(&tmp, &record);
        let r = DotnetPublishRecipe::new("publish", "MyApp.csproj");
        match r.execute(&c) {
            Ok(out) => {
                assert_eq!(out.recipe_name, "publish");
                assert!(!out.from_cache);
                let recorded = std::fs::read_to_string(&record).unwrap();
                assert_eq!(
                    recorded,
                    "publish\nMyApp.csproj\n-c\nRelease\nproject-root\n"
                );
            }
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }
}
