//! Core types, traits, and utilities for the must build system.

pub mod command;
pub mod error;
pub mod output;
pub mod paths;
pub mod traits;
pub mod types;

pub use command::{CommandOutput, run_command, run_status, shell_command, shell_program, shell_arg, shell_display};
pub use error::{Error, Result};
pub use output::{set_output_fn, clear_output_fn, print_output, print_error};
pub use paths::ensure_within_root;
pub use traits::{Cache, Recipe, Toolchain};
pub use types::{BuildContext, CacheKey, CacheLookup, CacheStrategy, RecipeOutput};
