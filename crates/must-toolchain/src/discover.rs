use crate::triple::Triple;
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
}
