use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::Cache;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheStrategy {
    Hash,
    Mtime,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub recipe: String,
    pub target: String,
    pub profile: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub enum CacheLookup {
    Hit,
    Miss,
    Stale,
}

pub struct BuildContext {
    pub project_root: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
    pub target: String,
    pub profile: String,
    pub env: HashMap<String, String>,
    pub dry_run: bool,
    pub parallelism: usize,
    pub cache: Option<Arc<dyn Cache>>,
}

impl std::fmt::Debug for BuildContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildContext")
            .field("project_root", &self.project_root)
            .field("cache_dir", &self.cache_dir)
            .field("log_dir", &self.log_dir)
            .field("target", &self.target)
            .field("profile", &self.profile)
            .field("env_count", &self.env.len())
            .field("dry_run", &self.dry_run)
            .field("parallelism", &self.parallelism)
            .field("cache", &self.cache.is_some())
            .finish()
    }
}

impl Clone for BuildContext {
    fn clone(&self) -> Self {
        Self {
            project_root: self.project_root.clone(),
            cache_dir: self.cache_dir.clone(),
            log_dir: self.log_dir.clone(),
            target: self.target.clone(),
            profile: self.profile.clone(),
            env: self.env.clone(),
            dry_run: self.dry_run,
            parallelism: self.parallelism,
            cache: self.cache.clone(),
        }
    }
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
            cache: None,
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[derive(Debug, Clone)]
pub struct RecipeOutput {
    pub recipe_name: String,
    pub from_cache: bool,
    pub outputs: Vec<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}
