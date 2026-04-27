use crate::triple::{Arch, Os, Triple};
use std::collections::HashMap;

/// Produce the environment variables needed to cross-compile Rust for `triple`.
///
/// Sets `CARGO_TARGET_<TRIPLE_UPPER>_LINKER` if `linker` is `Some`.
/// `TRIPLE_UPPER` is `triple.raw` with hyphens replaced by underscores and uppercased.
///
/// For the host sentinel (`triple.raw == "host"`), an empty map is returned since
/// no special cross-compilation environment is needed.
pub fn rust_cross_env(triple: &Triple, linker: Option<&str>) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if triple.is_host() {
        return env;
    }

    if let Some(linker_path) = linker {
        let key = format!(
            "CARGO_TARGET_{}_LINKER",
            triple.raw.replace('-', "_").to_uppercase()
        );
        env.insert(key, linker_path.to_string());
    }

    env
}

/// Produce `GOOS` and `GOARCH` environment variables for a triple.
///
/// Maps:
/// - `Arch::X86_64`  → `GOARCH=amd64`
/// - `Arch::Aarch64` → `GOARCH=arm64`
/// - `Arch::Riscv64` → `GOARCH=riscv64`
/// - `Os::Linux`     → `GOOS=linux`
/// - `Os::Macos`     → `GOOS=darwin`
/// - `Os::Windows`   → `GOOS=windows`
///
/// For the host sentinel, returns an empty map (no cross-compilation needed).
pub fn go_cross_env(triple: &Triple) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if triple.is_host() {
        return env;
    }

    let goarch = match triple.arch {
        Arch::X86_64 => "amd64",
        Arch::Aarch64 => "arm64",
        Arch::Riscv64 => "riscv64",
    };

    let goos = match triple.os {
        Os::Linux => "linux",
        Os::Macos => "darwin",
        Os::Windows => "windows",
    };

    env.insert("GOARCH".to_string(), goarch.to_string());
    env.insert("GOOS".to_string(), goos.to_string());

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triple::Triple;

    #[test]
    fn test_rust_cross_env_aarch64_with_linker() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let env = rust_cross_env(&triple, Some("aarch64-linux-gnu-gcc"));
        assert!(
            env.contains_key("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"),
            "expected CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER, got: {:?}",
            env
        );
        assert_eq!(
            env["CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER"],
            "aarch64-linux-gnu-gcc"
        );
    }

    #[test]
    fn test_rust_cross_env_no_linker_returns_empty() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let env = rust_cross_env(&triple, None);
        assert!(env.is_empty());
    }

    #[test]
    fn test_rust_cross_env_x86_64_with_linker() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        let env = rust_cross_env(&triple, Some("x86_64-linux-gnu-gcc"));
        assert!(env.contains_key("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER"));
        assert_eq!(
            env["CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER"],
            "x86_64-linux-gnu-gcc"
        );
    }

    #[test]
    fn test_rust_cross_env_host_returns_empty() {
        let triple = Triple::host();
        let env = rust_cross_env(&triple, Some("some-linker"));
        assert!(env.is_empty());
    }

    #[test]
    fn test_go_cross_env_aarch64_apple_darwin() {
        let triple = Triple::parse("aarch64-apple-darwin").unwrap();
        let env = go_cross_env(&triple);
        assert_eq!(env.get("GOARCH").map(String::as_str), Some("arm64"));
        assert_eq!(env.get("GOOS").map(String::as_str), Some("darwin"));
    }

    #[test]
    fn test_go_cross_env_x86_64_linux() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        let env = go_cross_env(&triple);
        assert_eq!(env.get("GOARCH").map(String::as_str), Some("amd64"));
        assert_eq!(env.get("GOOS").map(String::as_str), Some("linux"));
    }

    #[test]
    fn test_go_cross_env_x86_64_windows() {
        let triple = Triple::parse("x86_64-pc-windows-msvc").unwrap();
        let env = go_cross_env(&triple);
        assert_eq!(env.get("GOARCH").map(String::as_str), Some("amd64"));
        assert_eq!(env.get("GOOS").map(String::as_str), Some("windows"));
    }

    #[test]
    fn test_go_cross_env_host_returns_empty() {
        let triple = Triple::host();
        let env = go_cross_env(&triple);
        assert!(env.is_empty());
    }
}
