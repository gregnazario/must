use crate::error::Result;
use crate::types::{BuildContext, CacheKey, CacheLookup, CacheStrategy, RecipeOutput};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A build recipe that can be executed within a BuildContext.
///
/// Each recipe type (shell, rust-bin, go-bin, etc.) implements this trait.
pub trait Recipe: Send + Sync {
    fn name(&self) -> &str;
    fn deps(&self) -> &[String];
    fn inputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>>;
    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>>;
    fn cache_strategy(&self) -> CacheStrategy;
    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey>;
    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput>;
}

/// A toolchain that can run commands, optionally inside a container.
pub trait Toolchain: Send + Sync {
    fn target_triple(&self) -> &str;
    fn cc(&self) -> Option<&Path>;
    fn linker(&self) -> Option<&Path>;
    fn env(&self) -> HashMap<String, String>;
    fn execute(&self, cmd: Command) -> Result<Output>;
}

/// Persistent cache for recipe outputs.
pub trait Cache: Send + Sync {
    fn lookup(&self, key: &CacheKey) -> Result<CacheLookup>;
    fn store(&self, key: &CacheKey, root: &Path, outputs: &[PathBuf]) -> Result<()>;
    fn restore(&self, key: &CacheKey, root: &Path) -> Result<bool>;
    fn invalidate(&self, key: &CacheKey) -> Result<()>;
}
