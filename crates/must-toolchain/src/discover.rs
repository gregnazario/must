use crate::triple::{Os, Triple};
use std::path::PathBuf;
use std::process::Command;

/// Check if a Rust target is installed via rustup.
///
/// Runs `rustup target list --installed` and checks if `triple.raw` appears in the output.
/// Returns `false` (not an error) if rustup is not available or if the triple is `"host"`.
pub fn rust_target_installed(triple: &Triple) -> bool {
    if triple.is_host() {
        // Host target is always "installed".
        return true;
    }

    let output = match Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return false,
    };

    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().any(|line| line.trim() == triple.raw)
}

/// Get the install hint for a missing Rust target.
pub fn rust_install_hint(triple: &Triple) -> String {
    format!("Run: rustup target add {}", triple.raw)
}

/// Check if Go is installed by running `go version`.
pub fn go_installed() -> bool {
    Command::new("go")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the Go install hint.
pub fn go_install_hint() -> String {
    "Install Go from https://go.dev/dl/ or via your OS package manager".to_string()
}

/// Search PATH for a binary with the given name. Returns the full path if found.
fn which_in_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .iter()
        .flat_map(|p| std::env::split_paths(p))
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
}

/// Scan for a C cross-compiler for the given triple.
///
/// Looks for `<triple>-gcc` then `<triple>-clang` in:
///   - PATH (via which-style search)
///   - /usr/bin, /usr/local/bin
///   - /opt/homebrew/bin, /opt/homebrew/opt/llvm/bin (macOS)
///   - /usr/lib/llvm-14/bin through /usr/lib/llvm-17/bin (Ubuntu)
///
/// Returns the full path to the first found compiler, or `None`.
pub fn discover_c_compiler(triple: &Triple) -> Option<PathBuf> {
    let candidates = [
        format!("{}-gcc", triple.raw),
        format!("{}-clang", triple.raw),
    ];

    let search_dirs: Vec<PathBuf> = {
        let mut dirs = vec![
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/homebrew/opt/llvm/bin"),
        ];
        for version in 14..=17u32 {
            dirs.push(PathBuf::from(format!("/usr/lib/llvm-{}/bin", version)));
        }
        dirs
    };

    for candidate in &candidates {
        // First check PATH
        if let Some(path) = which_in_path(candidate) {
            return Some(path);
        }
        // Then check explicit directories
        for dir in &search_dirs {
            let path = dir.join(candidate);
            if path.exists() {
                return Some(path);
            }
        }
    }

    None
}

/// Check if any C compiler is available for this triple.
pub fn c_compiler_available(triple: &Triple) -> bool {
    if triple.is_host() {
        // For host, just check if `cc` or `gcc` exists in PATH
        which_in_path("cc").is_some() || which_in_path("gcc").is_some()
    } else {
        discover_c_compiler(triple).is_some()
    }
}

/// Return an OS-appropriate install hint for the C cross-compiler.
///
/// On Linux: suggests apt-get with a gcc cross package.
/// On macOS: suggests brew with FiloSottile/musl-cross or dockcross.
/// Fallback: generic install hint.
pub fn c_install_hint(triple: &Triple) -> String {
    let host = Triple::host();
    match host.os {
        Os::Linux => {
            // Derive short package name, e.g. aarch64-linux-gnu from aarch64-unknown-linux-gnu
            let short = triple
                .raw
                .replace("-unknown-", "-")
                .replace("-none-", "-");
            format!(
                "Run: sudo apt-get install gcc-{} (e.g. gcc-aarch64-linux-gnu)",
                short
            )
        }
        Os::Macos => format!(
            "Run: brew install FiloSottile/musl-cross/{} or use a dockcross container",
            triple.raw
        ),
        Os::Windows => format!(
            "Install a cross-compiler for {} or use cross = \"container\"",
            triple.raw
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triple::Triple;

    #[test]
    fn test_rust_install_hint_contains_rustup_target_add() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let hint = rust_install_hint(&triple);
        assert!(hint.contains("rustup target add"));
        assert!(hint.contains("aarch64-unknown-linux-gnu"));
    }

    #[test]
    fn test_rust_install_hint_host() {
        let triple = Triple::host();
        let hint = rust_install_hint(&triple);
        assert!(hint.contains("rustup target add"));
        assert!(hint.contains("host"));
    }

    #[test]
    fn test_go_install_hint_contains_go_dev() {
        let hint = go_install_hint();
        assert!(hint.contains("go.dev"));
    }

    // Note: rust_target_installed and go_installed are integration-style tests
    // that depend on the host environment. We don't assert a specific result,
    // but we verify they don't panic.
    #[test]
    fn test_rust_target_installed_does_not_panic() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        let _ = rust_target_installed(&triple);
    }

    #[test]
    fn test_rust_target_installed_host_returns_true() {
        let triple = Triple::host();
        assert!(rust_target_installed(&triple));
    }

    #[test]
    fn test_go_installed_does_not_panic() {
        let _ = go_installed();
    }

    #[test]
    fn test_discover_c_compiler_host_does_not_panic() {
        let triple = Triple::host();
        let _ = c_compiler_available(&triple);
    }

    #[test]
    fn test_c_install_hint_contains_triple() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        let hint = c_install_hint(&triple);
        assert!(
            hint.contains("aarch64"),
            "expected hint to contain 'aarch64', got: {}",
            hint
        );
    }
}
