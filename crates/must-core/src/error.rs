use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("config error in {path}: {message}")]
    Config { path: PathBuf, message: String },

    #[error("cycle detected in recipe graph: {cycle}")]
    CycleDetected { cycle: String },

    #[error("unknown recipe '{name}'")]
    UnknownRecipe { name: String },

    #[error("recipe '{name}' failed with exit code {code}:\n{stderr}")]
    RecipeFailed {
        name: String,
        code: i32,
        stderr: String,
    },

    #[error("toolchain not found for target '{target}': {hint}")]
    ToolchainNotFound { target: String, hint: String },

    #[error("{tool} not found: {hint}")]
    ToolNotFound { tool: String, hint: String },

    #[error("cache error: {0}")]
    Cache(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
