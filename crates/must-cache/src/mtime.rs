use must_core::{CacheLookup, Result};
use std::path::Path;
use std::time::SystemTime;

/// Returns CacheLookup::Hit if all outputs exist and are newer than all inputs.
/// Returns CacheLookup::Miss if any output is missing.
/// Returns CacheLookup::Stale if any input is newer than any output.
pub fn check_mtime(inputs: &[&Path], outputs: &[&Path]) -> Result<CacheLookup> {
    if outputs.is_empty() {
        // Phony recipe — always run
        return Ok(CacheLookup::Miss);
    }

    // Check all outputs exist
    for output in outputs {
        if !output.exists() {
            return Ok(CacheLookup::Miss);
        }
    }

    let max_input_mtime = inputs
        .iter()
        .filter_map(|p| p.metadata().ok()?.modified().ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let min_output_mtime = outputs
        .iter()
        .filter_map(|p| p.metadata().ok()?.modified().ok())
        .min()
        .unwrap_or(SystemTime::UNIX_EPOCH);

    if min_output_mtime > max_input_mtime {
        Ok(CacheLookup::Hit)
    } else {
        Ok(CacheLookup::Stale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    fn write(path: &std::path::Path, content: &str) {
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_miss_when_output_missing() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.txt");
        write(&input, "hello");
        let output = dir.path().join("output.txt");
        // output doesn't exist
        let result = check_mtime(&[&input], &[&output]).unwrap();
        assert!(matches!(result, CacheLookup::Miss));
    }

    #[test]
    fn test_hit_when_output_newer() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.txt");
        let output = dir.path().join("output.txt");
        write(&input, "hello");
        std::thread::sleep(Duration::from_millis(10));
        write(&output, "world");
        let result = check_mtime(&[&input], &[&output]).unwrap();
        assert!(matches!(result, CacheLookup::Hit));
    }

    #[test]
    fn test_stale_when_input_newer() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.txt");
        let output = dir.path().join("output.txt");
        write(&output, "old");
        std::thread::sleep(Duration::from_millis(10));
        write(&input, "newer");
        let result = check_mtime(&[&input], &[&output]).unwrap();
        assert!(matches!(result, CacheLookup::Stale));
    }
}
