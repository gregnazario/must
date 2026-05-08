//! Build engine that orchestrates recipe execution with dependency resolution.

pub mod env;
pub mod scheduler;

pub use env::{compose_env, compose_env_with_base};
pub use scheduler::{Engine, ExecutionReport, ExecutionResult, ProgressEvent};
