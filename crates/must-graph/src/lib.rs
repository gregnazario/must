//! Dependency graph with topological sorting and wave-based parallel execution.

pub mod dag;
pub use dag::{Dag, Wave};
