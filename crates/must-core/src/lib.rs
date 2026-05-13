//! Core types, traits, and utilities for the must build system.

pub mod command;
pub mod error;
pub mod output;
pub mod paths;
pub mod traits;
pub mod types;

pub use command::{
    CommandOutput, run_command, run_status, shell_arg, shell_command, shell_display, shell_program,
};
pub use error::{Error, Result};
pub use output::{clear_output_fn, print_error, print_output, set_output_fn};
pub use paths::ensure_within_root;
pub use traits::{Cache, Recipe, Toolchain};
pub use types::{BuildContext, CacheKey, CacheLookup, CacheStrategy, RecipeOutput};
