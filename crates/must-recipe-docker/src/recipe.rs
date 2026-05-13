use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheLookup, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    run_command,
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
    let out = run_command(
        &mut cmd,
        program,
        "Install Docker: https://docs.docker.com/get-docker/ or Podman: https://podman.io/",
    )?;
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

fn detect_runtime() -> &'static str {
    if Command::new("docker").arg("--version").output().is_ok() {
        "docker"
    } else {
        "podman"
    }
}

// ── DockerBuildRecipe ────────────────────────────────────────────────────────

pub struct DockerBuildRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub image: String,
    pub dockerfile: String,
    pub context: String,
    pub build_args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl DockerBuildRecipe {
    pub fn new(name: impl Into<String>, image: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            image: image.into(),
            dockerfile: ".".to_string(),
            context: ".".to_string(),
            build_args: Vec::new(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DockerBuildRecipe {
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
        flags.insert("image".to_string(), self.image.clone());
        flags.insert("dockerfile".to_string(), self.dockerfile.clone());
        flags.insert("context".to_string(), self.context.clone());
        for arg in &self.build_args {
            flags.insert(format!("build_arg_{}", arg), arg.clone());
        }
        Ok(make_cache_key(&self.name, "docker-build", ctx, &flags))
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
                    "[dry-run] {} build -t {} -f {} {}",
                    detect_runtime(),
                    self.image,
                    self.dockerfile,
                    self.context,
                ),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let rt = detect_runtime();
        let mut args = vec!["build", "-t", &self.image, "-f", &self.dockerfile];
        for arg in &self.build_args {
            args.push("--build-arg");
            args.push(arg);
        }
        args.push(&self.context);
        let mut result = run_cmd(rt, &args, ctx, &self.env, &ctx.project_root)?;
        result.recipe_name = self.name.clone();
        store_cache(&key, ctx);
        Ok(result)
    }
}

// ── DockerPushRecipe ─────────────────────────────────────────────────────────

pub struct DockerPushRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub image: String,
    pub env: HashMap<String, String>,
}

impl DockerPushRecipe {
    pub fn new(name: impl Into<String>, image: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            image: image.into(),
            env: HashMap::new(),
        }
    }
}

impl Recipe for DockerPushRecipe {
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
        flags.insert("image".to_string(), self.image.clone());
        Ok(make_cache_key(&self.name, "docker-push", ctx, &flags))
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![],
                stdout: format!("[dry-run] {} push {}", detect_runtime(), self.image),
                stderr: String::new(),
                duration_ms: 0,
            });
        }
        let rt = detect_runtime();
        let args = vec!["push", &self.image];
        let mut result = run_cmd(rt, &args, ctx, &self.env, &ctx.project_root)?;
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
    fn docker_build_cache_strategy_is_hash() {
        let r = DockerBuildRecipe::new("build", "myapp:latest");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn docker_build_name_and_image() {
        let r = DockerBuildRecipe::new("build", "myapp:latest");
        assert_eq!(r.name(), "build");
        assert_eq!(r.image, "myapp:latest");
    }

    #[test]
    fn docker_build_dry_run() {
        let r = DockerBuildRecipe::new("build", "myapp:latest");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("build"));
        assert!(out.stdout.contains("myapp:latest"));
    }

    #[test]
    fn docker_build_cache_hit() {
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
        let r = DockerBuildRecipe::new("build", "myapp:latest");
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
    fn docker_push_cache_strategy_is_never() {
        let r = DockerPushRecipe::new("push", "myapp:latest");
        assert_eq!(r.cache_strategy(), CacheStrategy::Never);
    }

    #[test]
    fn docker_push_dry_run() {
        let r = DockerPushRecipe::new("push", "myapp:latest");
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("push"));
        assert!(out.stdout.contains("myapp:latest"));
    }

    #[test]
    fn docker_build_key_includes_build_args() {
        let mut r1 = DockerBuildRecipe::new("r", "img");
        let mut r2 = DockerBuildRecipe::new("r", "img");
        r1.build_args = vec!["VERSION=1.0".to_string()];
        r2.build_args = vec!["VERSION=2.0".to_string()];
        assert_ne!(
            r1.cache_key(&ctx()).unwrap().hash,
            r2.cache_key(&ctx()).unwrap().hash
        );
    }

    #[test]
    fn docker_build_cache_store_and_second_hit() {
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
        let r = DockerBuildRecipe::new("build", "testimg:v1");
        let key = r.cache_key(&ctx).unwrap();
        let cache = must_cache::store::DiskCache::open(&ctx.cache_dir).unwrap();
        cache.store(&key, &[]).unwrap();
        drop(cache);
        let out1 = r.execute(&ctx);
        match out1 {
            Ok(o) => {
                assert!(o.from_cache);
            }
            Err(must_core::Error::ToolNotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn docker_build_with_custom_dockerfile_and_context() {
        let mut r = DockerBuildRecipe::new("custom", "myapp:v2");
        r.dockerfile = "Dockerfile.prod".to_string();
        r.context = "deploy".to_string();
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("Dockerfile.prod"));
        assert!(out.stdout.contains("deploy"));
    }

    #[test]
    fn docker_build_with_build_args_dry_run() {
        let mut r = DockerBuildRecipe::new("build", "app:latest");
        r.build_args = vec!["VERSION=1.0".to_string(), "ENV=prod".to_string()];
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
        assert!(out.stdout.contains("app:latest"));
    }

    #[test]
    fn docker_push_name_and_image() {
        let r = DockerPushRecipe::new("push", "myregistry/myapp:v1");
        assert_eq!(r.name(), "push");
        assert_eq!(r.image, "myregistry/myapp:v1");
    }

    #[test]
    fn docker_push_inputs_outputs_empty() {
        let r = DockerPushRecipe::new("push", "img");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
        assert!(r.outputs(&ctx()).unwrap().is_empty());
    }

    #[test]
    fn docker_push_deps_empty() {
        let r = DockerPushRecipe::new("push", "img");
        assert!(r.deps().is_empty());
    }

    #[test]
    fn docker_build_with_env_dry_run() {
        let mut r = DockerBuildRecipe::new("build", "app:test");
        r.env = HashMap::from([("DOCKER_BUILDKIT".to_string(), "1".to_string())]);
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn docker_push_with_env_dry_run() {
        let mut r = DockerPushRecipe::new("push", "app:test");
        r.env = HashMap::from([(
            "DOCKER_HOST".to_string(),
            "tcp://localhost:2375".to_string(),
        )]);
        let mut c = ctx();
        c.dry_run = true;
        let out = r.execute(&c).unwrap();
        assert!(out.stdout.contains("dry-run"));
    }

    #[test]
    fn docker_build_inputs_empty() {
        let r = DockerBuildRecipe::new("build", "img");
        assert!(r.inputs(&ctx()).unwrap().is_empty());
    }
}
