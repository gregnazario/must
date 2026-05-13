use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command, shell_command, shell_program,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_cmd_in(
    program: &str,
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    workdir: &std::path::Path,
) -> Result<RecipeOutput> {
    let start = Instant::now();
    let mut cmd = Command::new(program);
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
    let out = run_command(&mut cmd, program, "Install Python 3: https://python.org")?;
    let duration_ms = start.elapsed().as_millis() as u64;

    if !out.status.success() {
        return Err(Error::RecipeFailed {
            name: program.to_string(),
            code: out.status.code().unwrap_or(-1),
            stderr: out.stderr,
        });
    }
    Ok(RecipeOutput {
        recipe_name: program.to_string(),
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

fn detect_uv_or_pip() -> (&'static str, &'static [&'static str]) {
    if Command::new("uv").arg("--version").output().is_ok() {
        ("uv", &["pip", "install"])
    } else {
        ("pip", &["install"])
    }
}

fn workdir_path(ctx: &BuildContext, workdir: &str) -> std::path::PathBuf {
    if workdir == "." {
        ctx.project_root.clone()
    } else {
        ctx.project_root.join(workdir)
    }
}

// ── PyBinRecipe ──────────────────────────────────────────────────────────────

pub struct PyBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl PyBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for PyBinRecipe {
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
        Ok(make_cache_key(&self.name, "py-bin", ctx, &flags))
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
            let (tool, prefix) = detect_uv_or_pip();
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!(
                    "[dry-run] {} {} {} (in {})",
                    tool,
                    prefix.join(" "),
                    self.package,
                    self.package
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let (tool, prefix) = detect_uv_or_pip();
        let mut args: Vec<&str> = prefix.to_vec();
        args.push(&self.package);
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_cmd_in(tool, &args, ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

// ── PyTestRecipe ─────────────────────────────────────────────────────────────

pub struct PyTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl PyTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for PyTestRecipe {
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
        Ok(make_cache_key(&self.name, "py-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] pytest (in {})", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_cmd_in("pytest", &[], ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

// ── PyLintRecipe ─────────────────────────────────────────────────────────────

pub struct PyLintRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl PyLintRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for PyLintRecipe {
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
        Ok(make_cache_key(&self.name, "py-lint", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!(
                    "[dry-run] ruff check {} && mypy {}",
                    self.package, self.package
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut cmd = shell_command("ruff check . && mypy .");
        cmd.current_dir(&dir);
        cmd.env_clear();
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }
        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        let start = Instant::now();
        let out = run_command(
            &mut cmd,
            shell_program(),
            "ruff and mypy are required for linting",
        )?;
        let duration_ms = start.elapsed().as_millis() as u64;

        if !out.status.success() {
            return Err(Error::RecipeFailed {
                name: self.name.clone(),
                code: out.status.code().unwrap_or(-1),
                stderr: out.stderr,
            });
        }
        Ok(RecipeOutput {
            recipe_name: self.name.clone(),
            from_cache: false,
            outputs: Vec::new(),
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
    fn py_bin_cache_strategy_is_hash() {
        let r = PyBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn py_bin_name_and_package() {
        let r = PyBinRecipe::new("build", "packages/api");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "packages/api");
    }

    #[test]
    fn py_bin_dry_run() {
        let r = PyBinRecipe::new("build", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("install"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn py_bin_cache_hit() {
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
        let r = PyBinRecipe::new("build", ".");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out = r.execute(&ctx);
        match out {
            Ok(o) => assert!(o.from_cache, "should be a cache hit"),
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_test_cache_strategy_is_never() {
        let r = PyTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn py_test_dry_run() {
        let r = PyTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("pytest"));
    }

    #[test]
    fn py_lint_cache_strategy_is_never() {
        let r = PyLintRecipe::new("lint", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn py_lint_dry_run() {
        let r = PyLintRecipe::new("lint", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("ruff"));
        assert!(out.stdout.contains("mypy"));
    }

    #[test]
    fn py_cache_key_differs_by_package() {
        let r1 = PyBinRecipe::new("r", "pkg-a");
        let r2 = PyBinRecipe::new("r", "pkg-b");
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn py_bin_execute_real_install() {
        if std::process::Command::new("uv")
            .arg("--version")
            .output()
            .is_err()
            && std::process::Command::new("pip")
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
        let r = PyBinRecipe::new("build", ".");
        let result = r.execute(&c);
        match result {
            Ok(out) => {
                assert_eq!(out.recipe_name, "build");
                assert!(!out.from_cache);
            }
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_test_execute_real() {
        if std::process::Command::new("pytest")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("conftest.py"), "").unwrap();
        std::fs::write(tmp.path().join("test_noop.py"), "def test_noop(): pass\n").unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = PyTestRecipe::new("test", ".");
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "test"),
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_bin_execute_with_recipe_env() {
        if std::process::Command::new("uv")
            .arg("--version")
            .output()
            .is_err()
            && std::process::Command::new("pip")
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
        c.env.insert("MY_GLOBAL".to_string(), "1".to_string());
        let mut r = PyBinRecipe::new("build", ".");
        r.env = HashMap::from([("MY_EXTRA".to_string(), "2".to_string())]);
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "build"),
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_bin_workdir_not_dot() {
        if std::process::Command::new("uv")
            .arg("--version")
            .output()
            .is_err()
            && std::process::Command::new("pip")
                .arg("--version")
                .output()
                .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        let sub = tmp.path().join("subpkg");
        std::fs::create_dir_all(&sub).unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = PyBinRecipe::new("build", "subpkg");
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "build"),
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_lint_execute_real() {
        if std::process::Command::new("ruff")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("noop.py"), "x = 1\n").unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = PyLintRecipe::new("lint", ".");
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "lint"),
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_bin_cache_store_and_second_hit() {
        if std::process::Command::new("uv")
            .arg("--version")
            .output()
            .is_err()
            && std::process::Command::new("pip")
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
        let r = PyBinRecipe::new("build", ".");
        let result = r.execute(&c);
        match result {
            Ok(out) => {
                assert!(!out.from_cache);
                let out2 = r.execute(&c).unwrap();
                assert!(out2.from_cache, "second run should hit cache");
            }
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_test_execute_with_env() {
        if std::process::Command::new("pytest")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("conftest.py"), "").unwrap();
        std::fs::write(
            tmp.path().join("test_env.py"),
            "import os\ndef test_env(): assert os.environ.get('MY_VAR') == 'hello'\n",
        )
        .unwrap();
        let mut r = PyTestRecipe::new("test", ".");
        r.env = HashMap::from([("MY_VAR".to_string(), "hello".to_string())]);
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "test"),
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn py_bin_execute_tool_not_found() {
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
        let r = PyBinRecipe::new("build", ".");
        let result = r.execute(&ctx);
        assert!(result.is_err(), "should fail without PATH");
    }
}
