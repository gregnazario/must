use crate::triple::{Arch, Os, Triple};
use std::collections::HashMap;
use std::path::Path;

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

/// Produce CC, CXX, AR, LD env vars for C cross-compilation.
///
/// For the host triple (or when compiler is None), returns standard host tool names.
/// For cross triples, uses the provided compiler path to derive the full toolchain.
///
/// Derivation rules for cross compilers:
/// - `CXX`: replace `-gcc` → `-g++`, `-clang` → `-clang++` in the filename
/// - `AR`: replace the compiler binary name's suffix with `-ar`; fall back to `ar`
/// - `LD`: replace the compiler binary name's suffix with `-ld`; fall back to `ld`
pub fn c_cross_env(triple: &Triple, compiler: Option<&Path>) -> HashMap<String, String> {
    let mut env = HashMap::new();

    if let Some(cc_path) = compiler {
        let cc_str = cc_path.to_string_lossy().to_string();

        // Derive CXX from CC filename
        let file_name = cc_path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let cxx_name = if file_name.ends_with("-gcc") {
            file_name.trim_end_matches("-gcc").to_string() + "-g++"
        } else if file_name.ends_with("-clang") {
            file_name.trim_end_matches("-clang").to_string() + "-clang++"
        } else {
            // append ++ to the stem
            let stem = cc_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file_name.clone());
            stem + "++"
        };

        let cxx_path = cc_path.with_file_name(&cxx_name);
        env.insert("CXX".to_string(), cxx_path.to_string_lossy().to_string());

        // Derive AR: replace the gcc/clang suffix with -ar
        let ar_name = if file_name.ends_with("-gcc") {
            file_name.trim_end_matches("-gcc").to_string() + "-ar"
        } else if file_name.ends_with("-clang") {
            file_name.trim_end_matches("-clang").to_string() + "-ar"
        } else {
            "ar".to_string()
        };
        let ar_candidate = cc_path.with_file_name(&ar_name);
        let ar_val = if ar_candidate.exists() {
            ar_candidate.to_string_lossy().to_string()
        } else {
            ar_name
        };
        env.insert("AR".to_string(), ar_val);

        // Derive LD: replace the gcc/clang suffix with -ld
        let ld_name = if file_name.ends_with("-gcc") {
            file_name.trim_end_matches("-gcc").to_string() + "-ld"
        } else if file_name.ends_with("-clang") {
            file_name.trim_end_matches("-clang").to_string() + "-ld"
        } else {
            "ld".to_string()
        };
        let ld_candidate = cc_path.with_file_name(&ld_name);
        let ld_val = if ld_candidate.exists() {
            ld_candidate.to_string_lossy().to_string()
        } else {
            ld_name
        };
        env.insert("LD".to_string(), ld_val);

        env.insert("CC".to_string(), cc_str);
    } else {
        // Host defaults
        env.insert("CC".to_string(), "cc".to_string());
        env.insert("CXX".to_string(), "c++".to_string());
        env.insert("AR".to_string(), "ar".to_string());
        env.insert("LD".to_string(), "ld".to_string());
    }

    // CFLAGS: empty by default, let caller override
    // For cross Linux targets, no sysroot by default (linker handles it)
    env.insert("CFLAGS".to_string(), "".to_string());

    let _ = triple; // triple is available for future use (e.g., target-specific flags)

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

    #[test]
    fn test_c_cross_env_none_compiler_host_defaults() {
        let triple = Triple::host();
        let env = c_cross_env(&triple, None);
        assert_eq!(env.get("CC").map(String::as_str), Some("cc"));
        assert_eq!(env.get("AR").map(String::as_str), Some("ar"));
    }

    #[test]
    fn test_c_cross_env_with_clang_compiler() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let compiler = Path::new("/usr/bin/aarch64-linux-gnu-clang");
        let env = c_cross_env(&triple, Some(compiler));
        assert_eq!(
            env.get("CC").map(String::as_str),
            Some("/usr/bin/aarch64-linux-gnu-clang")
        );
        let cxx = env.get("CXX").expect("CXX should be set");
        assert!(
            cxx.ends_with("-clang++"),
            "expected CXX to end with -clang++, got: {cxx}"
        );
        let ar = env.get("AR").expect("AR should be set");
        assert!(
            ar.ends_with("-ar") || ar == "ar",
            "expected AR to end with -ar or be 'ar', got: {ar}"
        );
    }

    #[test]
    fn test_c_cross_env_with_generic_compiler_name() {
        // Compiler with no -gcc/-clang suffix → fallback to "ar" / "<stem>++"
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let compiler = Path::new("/usr/local/bin/mycc");
        let env = c_cross_env(&triple, Some(compiler));
        assert_eq!(
            env.get("CC").map(String::as_str),
            Some("/usr/local/bin/mycc")
        );
        // CXX should be the stem + "++"
        let cxx = env.get("CXX").expect("CXX");
        assert!(
            cxx.contains("++"),
            "expected CXX to contain '++', got: {cxx}"
        );
        // AR should fall back to "ar"
        assert_eq!(env.get("AR").map(String::as_str), Some("ar"));
        assert_eq!(env.get("LD").map(String::as_str), Some("ld"));
    }

    #[test]
    fn test_c_cross_env_cflags_always_present() {
        let triple = Triple::host();
        let env = c_cross_env(&triple, None);
        assert!(env.contains_key("CFLAGS"), "CFLAGS should always be set");
    }

    #[test]
    fn test_rust_cross_env_with_linker_produces_uppercased_key() {
        // Verify the key format: CARGO_TARGET_<TRIPLE_UPPER>_LINKER
        let triple = Triple::parse("x86_64-unknown-linux-musl").unwrap();
        let env = rust_cross_env(&triple, Some("/usr/bin/musl-gcc"));
        let key = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER";
        assert!(
            env.contains_key(key),
            "expected key {key}, got: {:?}",
            env.keys().collect::<Vec<_>>()
        );
        assert_eq!(env[key], "/usr/bin/musl-gcc");
    }

    #[test]
    fn test_go_cross_env_riscv64() {
        // riscv64 is in the parse table (Arch::Riscv64 exists in triple.rs)
        if let Ok(triple) = Triple::parse("riscv64-unknown-linux-gnu") {
            let env = go_cross_env(&triple);
            assert_eq!(env.get("GOARCH").map(String::as_str), Some("riscv64"));
            assert_eq!(env.get("GOOS").map(String::as_str), Some("linux"));
        }
    }

    #[test]
    fn test_c_cross_env_with_compiler_path() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let compiler = Path::new("/usr/bin/aarch64-linux-gnu-gcc");
        let env = c_cross_env(&triple, Some(compiler));
        assert_eq!(
            env.get("CC").map(String::as_str),
            Some("/usr/bin/aarch64-linux-gnu-gcc")
        );
        let cxx = env.get("CXX").expect("CXX should be set");
        assert!(
            cxx.contains("g++") || cxx.contains("clang++"),
            "expected CXX to contain g++ or clang++, got: {}",
            cxx
        );
        assert!(env.contains_key("AR"), "AR should be set");
    }
}
