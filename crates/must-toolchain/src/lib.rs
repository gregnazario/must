pub mod discover;
pub mod local_toolchain;
pub mod triple;

pub use discover::{
    c_compiler_available, c_install_hint, discover_c_compiler, go_install_hint, go_installed,
    rust_install_hint, rust_target_installed,
};
pub use local_toolchain::{c_cross_env, go_cross_env, rust_cross_env};
pub use triple::{Arch, Os, Triple};
