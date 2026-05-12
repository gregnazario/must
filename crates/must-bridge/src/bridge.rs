use must_core::{
    BuildContext, CacheKey, CacheStrategy, Error, Recipe, RecipeOutput, Result, run_command,
    shell_command, shell_program,
};
use std::path::PathBuf;
use std::time::Instant;

pub struct BridgeRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub tool: String,
    pub script: String,
}

impl BridgeRecipe {
    pub fn new(name: impl Into<String>, tool: impl Into<String>, script: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            tool: tool.into(),
            script: script.into(),
        }
    }
}

impl Recipe for BridgeRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn outputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }

    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Never
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let key_str = format!("bridge:{}:{}:{}", self.name, ctx.target, ctx.profile);
        let hash = must_cache::store::hash_string(&key_str);
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash,
        })
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: Vec::new(),
                stdout: format!(
                    "[dry-run] would run via {}: {}",
                    self.tool,
                    self.script
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        let start = Instant::now();
        let mut cmd = shell_command(&self.script);
        cmd.current_dir(&ctx.project_root);
        let out = run_command(
            &mut cmd,
            shell_program(),
            &format!("a shell is required (bridge delegates to {})", self.tool),
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
    use std::collections::HashMap;

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
    fn name_and_tool() {
        let r = BridgeRecipe::new("build", "make", "make build");
        assert_eq!(r.name(), "build");
        assert_eq!(r.tool, "make");
    }

    #[test]
    fn cache_strategy_is_never() {
        let r = BridgeRecipe::new("build", "make", "make");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn dry_run_skips_execution() {
        let r = BridgeRecipe::new("build", "make", "make build");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("make"));
        assert_eq!(out.duration_ms, 0);
    }

    #[test]
    fn execute_runs_script() {
        let r = BridgeRecipe::new("greet", "make", "echo hello-bridge");
        let out = r.execute(&ctx()).unwrap();
        assert!(out.stdout.contains("hello-bridge"));
        assert!(!out.from_cache);
    }

    #[test]
    fn execute_fails_on_nonzero() {
        let r = BridgeRecipe::new("bad", "make", "exit 1");
        let result = r.execute(&ctx());
        assert!(result.is_err());
    }

    #[test]
    fn inputs_outputs_empty() {
        let r = BridgeRecipe::new("build", "make", "make");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn deps_empty_by_default() {
        let r = BridgeRecipe::new("build", "make", "make");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn cache_key_is_stable() {
        let r = BridgeRecipe::new("build", "make", "make build");
        let key = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.recipe, "build");
        let key2 = r.cache_key(&ctx()).unwrap();
        assert_eq!(key.hash, key2.hash);
    }
}
