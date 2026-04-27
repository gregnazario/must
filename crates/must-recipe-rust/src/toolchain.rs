use std::process::Command;

/// Returns the output of `rustc -V`, e.g. "rustc 1.78.0 (9b00956e5 2024-04-29)".
/// Falls back to "rustc unknown" on error.
pub fn rustc_version() -> String {
    Command::new("rustc")
        .arg("-V")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "rustc unknown".to_string())
}
