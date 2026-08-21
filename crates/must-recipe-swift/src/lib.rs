use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

fn run_swift(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
    workdir: &std::path::Path,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("swift");
    let start = Instant::now();
    let mut cmd = Command::new("swift");
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
        "swift",
        "Install Swift: https://swift.org/download/ or Xcode from the App Store",
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
        let _ = cache.store(key, &[]);
    } else if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
        let _ = Cache::store(&cache, key, &[]);
    }
}

fn swift_version() -> String {
    Command::new("swift")
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

fn workdir_path(ctx: &BuildContext, workdir: &str) -> std::path::PathBuf {
    if workdir == "." {
        ctx.project_root.clone()
    } else {
        ctx.project_root.join(workdir)
    }
}

pub struct SwiftBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl SwiftBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for SwiftBinRecipe {
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
        flags.insert("swift_version".to_string(), swift_version());
        Ok(make_cache_key(&self.name, "swift-bin", ctx, &self.env, &flags))
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
                stdout: format!("[dry-run] swift build -c release (in {})", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_swift(&["build", "-c", "release"], ctx, &self.env, &dir)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

pub struct SwiftTestRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub package: String,
    pub env: HashMap<String, String>,
}

impl SwiftTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            package: package.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for SwiftTestRecipe {
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
        Ok(make_cache_key(&self.name, "swift-test", ctx, &self.env, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] swift test (in {})", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let dir = workdir_path(ctx, &self.package);
        let mut result = run_swift(&["test"], ctx, &self.env, &dir)?;
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
    fn swift_bin_cache_strategy_is_hash() {
        let r = SwiftBinRecipe::new("build", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn swift_bin_name_and_package() {
        let r = SwiftBinRecipe::new("build", "MyPackage");
        assert_eq!(r.name(), "build");
        assert_eq!(r.package, "MyPackage");
    }

    #[test]
    fn swift_bin_deps_empty_by_default() {
        let r = SwiftBinRecipe::new("build", ".");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn swift_bin_inputs_outputs_empty() {
        let r = SwiftBinRecipe::new("build", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn swift_bin_dry_run() {
        let r = SwiftBinRecipe::new("build", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("swift build"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn swift_bin_cache_hit() {
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
        let r = SwiftBinRecipe::new("build", ".");
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
    fn swift_bin_cache_key_differs_by_package() {
        let r1 = SwiftBinRecipe::new("r", "pkg-a");
        let r2 = SwiftBinRecipe::new("r", "pkg-b");
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn swift_test_cache_strategy_is_never() {
        let r = SwiftTestRecipe::new("test", ".");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn swift_test_name_and_package() {
        let r = SwiftTestRecipe::new("test", "MyPackage");
        assert_eq!(r.name(), "test");
        assert_eq!(r.package, "MyPackage");
    }

    #[test]
    fn swift_test_dry_run() {
        let r = SwiftTestRecipe::new("test", ".");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("swift test"));
    }

    #[test]
    fn swift_bin_execute_real() {
        if std::process::Command::new("swift")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Package.swift"),
            r#"// swift-tools-version:5.9
import PackageDescription
let package = Package(name: "TestPkg", targets: [.executableTarget(name: "TestPkg")])
"#,
        )
        .unwrap();
        let sources = tmp.path().join("Sources/TestPkg");
        std::fs::create_dir_all(&sources).unwrap();
        std::fs::write(sources.join("main.swift"), "print(\"hello\")\n").unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = SwiftBinRecipe::new("build", ".");
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
    fn swift_test_execute_real() {
        if std::process::Command::new("swift")
            .arg("--version")
            .output()
            .is_err()
        {
            return;
        }
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Package.swift"),
            r#"// swift-tools-version:5.9
import PackageDescription
let package = Package(name: "TestPkg", targets: [.testTarget(name: "TestPkgTests")])
"#,
        )
        .unwrap();
        let tests = tmp.path().join("Tests/TestPkgTests");
        std::fs::create_dir_all(&tests).unwrap();
        std::fs::write(
            tests.join("TestPkgTests.swift"),
            r#"import XCTest
final class TestPkgTests: XCTestCase { func testExample() { XCTAssertEqual(1, 1) } }
"#,
        )
        .unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let r = SwiftTestRecipe::new("test", ".");
        let result = r.execute(&c);
        match result {
            Ok(out) => assert_eq!(out.recipe_name, "test"),
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(must_core::Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn swift_bin_tool_not_found() {
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
        let r = SwiftBinRecipe::new("build", ".");
        assert!(r.execute(&ctx).is_err());
    }

    #[test]
    fn swift_bin_cache_store_and_hit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let mut c = ctx_with_path();
        c.project_root = tmp.path().to_owned();
        c.cache_dir = tmp.path().join(".must/cache");
        let key = {
            let r = SwiftBinRecipe::new("build", ".");
            r.cache_key(&c).unwrap()
        };
        let cache = must_cache::store::DiskCache::open(&c.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let r = SwiftBinRecipe::new("build", ".");
        let out = r.execute(&c);
        match out {
            Ok(o) => {
                assert!(o.from_cache);
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn swift_test_inputs_outputs_empty() {
        let r = SwiftTestRecipe::new("test", ".");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }
}
