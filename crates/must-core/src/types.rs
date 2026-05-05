use std::collections::HashMap;
use std::path::PathBuf;

/// Strategy used to determine whether a recipe needs to re-run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStrategy {
    /// Compare SHA-256 of inputs vs stored hash. Branch-switch-safe.
    Hash,
    /// Compare max(input mtime) vs min(output mtime). Faster but not branch-safe.
    Mtime,
    /// Always execute; skip cache lookup entirely.
    Never,
}

/// Opaque cache key for a recipe execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub recipe: String,
    pub target: String,
    pub profile: String,
    pub hash: String,
}

/// Result of a cache lookup.
#[derive(Debug, Clone)]
pub enum CacheLookup {
    Hit,
    Miss,
    Stale,
}

/// Context passed to every recipe execution.
#[derive(Debug, Clone)]
pub struct BuildContext {
    pub project_root: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub target: String,
    pub profile: String,
    pub env: HashMap<String, String>,
    pub dry_run: bool,
    pub parallelism: usize,
}

impl BuildContext {
    pub fn new(project_root: PathBuf) -> Self {
        let cache_dir = project_root.join(".mustfile").join("cache");
        let log_dir = project_root.join(".mustfile").join("logs");
        Self {
            project_root,
            cache_dir,
            log_dir,
            target: "host".to_string(),
            profile: "default".to_string(),
            env: HashMap::new(),
            dry_run: false,
            parallelism: num_cpus(),
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// Output produced by a recipe execution.
#[derive(Debug, Clone)]
pub struct RecipeOutput {
    pub recipe_name: String,
    pub from_cache: bool,
    pub outputs: Vec<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}
