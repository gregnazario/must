use must_cache::hash::compute_hash;
use must_core::{BuildContext, CacheKey, CacheStrategy, Error, Recipe, RecipeOutput, Result};
use must_toolchain::{go_cross_env, go_install_hint, go_installed, Triple};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

// ── Shared helpers ─────────────────────────────────────────────────────────────

/// Run `go version` and return the first line of output, or `"unknown"` if go is not found.
fn go_version() -> String {
    Command::new("go")
        .arg("version")
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

fn run_go(
    args: &[&str],
    ctx: &BuildContext,
    extra_env: &HashMap<String, String>,
) -> Result<RecipeOutput> {
    let name = args.first().copied().unwrap_or("go");
    let start = Instant::now();
    let mut cmd = Command::new("go");
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
        &[], // Go tracks its own deps
        &env_btree,
        &go_version(),
        extra_flags,
    );
    CacheKey {
        recipe: recipe_name.to_string(),
        target: ctx.target.clone(),
        profile: ctx.profile.clone(),
        hash,
    }
}

// ── GoBinRecipe ───────────────────────────────────────────────────────────────

pub struct GoBinRecipe {
    pub name: String,
    pub package: String,
    pub deps: Vec<String>,
    pub ldflags: Option<String>,
    pub build_tags: Vec<String>,
    pub env: HashMap<String, String>,
}

impl GoBinRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            package: package.into(),
            deps: Vec::new(),
            ldflags: None,
            build_tags: Vec::new(),
            env: HashMap::new(),
        }
    }

    fn extra_flags(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("package".to_string(), self.package.clone());
        m.insert(
            "ldflags".to_string(),
            self.ldflags.as_deref().unwrap_or("").to_string(),
        );
        m.insert("tags".to_string(), self.build_tags.join(","));
        m
    }
}

impl Recipe for GoBinRecipe {
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
        Ok(make_cache_key(&self.name, "go-bin", ctx, &self.extra_flags()))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        // Resolve the target triple (for cross-compilation env)
        let triple = if ctx.target != "host" {
            Triple::parse(&ctx.target)?
        } else {
            Triple::host()
        };

        // Verify Go is installed
        if !go_installed() {
            return Err(Error::ToolchainNotFound {
                target: ctx.target.clone(),
                hint: go_install_hint(),
            });
        }

        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!("[dry-run] go build {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        // Build argument list: go build [-tags <t>] [-ldflags <f>] <package>
        let mut args: Vec<String> = vec!["build".to_string()];

        if !self.build_tags.is_empty() {
            args.push("-tags".to_string());
            args.push(self.build_tags.join(","));
        }

        if let Some(ref flags) = self.ldflags {
            args.push("-ldflags".to_string());
            args.push(flags.clone());
        }

        args.push(self.package.clone());

        // Collect extra env: recipe env + cross-compile env
        let mut extra_env: HashMap<String, String> = self.env.clone();
        for (k, v) in go_cross_env(&triple) {
            extra_env.insert(k, v);
        }

        let args_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut result = run_go(&args_refs, ctx, &extra_env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

// ── GoTestRecipe ──────────────────────────────────────────────────────────────

pub struct GoTestRecipe {
    pub name: String,
    pub package: String,
    pub deps: Vec<String>,
    pub env: HashMap<String, String>,
}

impl GoTestRecipe {
    pub fn new(name: impl Into<String>, package: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            package: package.into(),
            deps: Vec::new(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for GoTestRecipe {
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
        Ok(make_cache_key(&self.name, "go-test", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if !go_installed() {
            return Err(Error::ToolchainNotFound {
                target: ctx.target.clone(),
                hint: go_install_hint(),
            });
        }

        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!("[dry-run] go test {}", self.package),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let args = &["test", &self.package];
        let mut result = run_go(args, ctx, &self.env)?;
        result.recipe_name = self.name.clone();
        Ok(result)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_build_context() -> BuildContext {
        BuildContext {
            project_root: std::path::PathBuf::from("/tmp/test"),
            cache_dir: std::path::PathBuf::from("/tmp/cache"),
            target: "host".to_string(),
            profile: "debug".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: 1,
        }
    }

    #[test]
    fn test_go_bin_recipe_cache_strategy_is_hash() {
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        assert!(matches!(recipe.cache_strategy(), CacheStrategy::Hash));
    }

    #[test]
    fn test_go_test_recipe_cache_strategy_is_never() {
        let recipe = GoTestRecipe::new("my-test", "./...");
        assert!(matches!(recipe.cache_strategy(), CacheStrategy::Never));
    }

    #[test]
    fn test_go_bin_recipe_deps_returns_expected_slice() {
        let mut recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        recipe.deps = vec!["dep-a".to_string(), "dep-b".to_string()];
        assert_eq!(recipe.deps(), &["dep-a", "dep-b"]);
    }

    #[test]
    fn test_go_bin_recipe_empty_deps() {
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        assert!(recipe.deps().is_empty());
    }

    #[test]
    fn test_go_test_recipe_deps_returns_expected_slice() {
        let mut recipe = GoTestRecipe::new("my-test", "./...");
        recipe.deps = vec!["build".to_string()];
        assert_eq!(recipe.deps(), &["build"]);
    }

    #[test]
    fn test_go_bin_recipe_inputs_is_empty() {
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        let ctx = make_build_context();
        assert!(recipe.inputs(&ctx).unwrap().is_empty());
    }

    #[test]
    fn test_go_bin_recipe_outputs_is_empty() {
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        let ctx = make_build_context();
        assert!(recipe.outputs(&ctx).unwrap().is_empty());
    }

    #[test]
    fn test_go_test_recipe_inputs_is_empty() {
        let recipe = GoTestRecipe::new("my-test", "./...");
        let ctx = make_build_context();
        assert!(recipe.inputs(&ctx).unwrap().is_empty());
    }

    #[test]
    fn test_go_test_recipe_outputs_is_empty() {
        let recipe = GoTestRecipe::new("my-test", "./...");
        let ctx = make_build_context();
        assert!(recipe.outputs(&ctx).unwrap().is_empty());
    }

    #[test]
    fn test_go_bin_recipe_name() {
        let recipe = GoBinRecipe::new("server-build", "./cmd/server");
        assert_eq!(recipe.name(), "server-build");
    }

    #[test]
    fn test_go_test_recipe_name() {
        let recipe = GoTestRecipe::new("unit-tests", "./...");
        assert_eq!(recipe.name(), "unit-tests");
    }

    #[test]
    fn test_go_bin_execute_dry_run() {
        // dry_run is reached only when go is installed; otherwise we get ToolchainNotFound.
        // Both outcomes are acceptable — we just verify no panic and correct branch behavior.
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        let mut ctx = make_build_context();
        ctx.dry_run = true;
        match recipe.execute(&ctx) {
            Ok(out) => {
                assert!(out.stdout.contains("dry-run"));
                assert_eq!(out.duration_ms, 0);
            }
            Err(Error::ToolchainNotFound { .. }) => {} // go not installed — acceptable
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_go_test_execute_dry_run() {
        let recipe = GoTestRecipe::new("my-tests", "./...");
        let mut ctx = make_build_context();
        ctx.dry_run = true;
        match recipe.execute(&ctx) {
            Ok(out) => {
                assert!(out.stdout.contains("dry-run"));
                assert_eq!(out.duration_ms, 0);
            }
            Err(Error::ToolchainNotFound { .. }) => {} // go not installed — acceptable
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn test_go_bin_execute_not_installed_returns_toolchain_error() {
        // Test the path when go is NOT installed. We can't force go to be unavailable,
        // but we can test that if it returns a ToolchainNotFound error, it contains
        // target and hint information. Skip if go is actually installed.
        let recipe = GoBinRecipe::new("my-bin", "./cmd/server");
        let ctx = make_build_context();
        if go_installed() {
            return; // go is installed, can't test this path
        }
        match recipe.execute(&ctx) {
            Err(Error::ToolchainNotFound { target, hint }) => {
                assert_eq!(target, "host");
                assert!(!hint.is_empty());
            }
            other => panic!("expected ToolchainNotFound, got: {other:?}"),
        }
    }
}
