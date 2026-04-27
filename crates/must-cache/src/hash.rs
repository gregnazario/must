use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// Compute a deterministic cache key hash for a recipe.
///
/// Inputs hashed (in stable order):
///   1. recipe_name
///   2. recipe_type (string tag like "rust-bin", "shell", etc.)
///   3. sorted input file contents (path → SHA-256 of content)
///   4. env vars that affect the build (sorted by key)
///   5. toolchain_id (e.g. "rustc 1.78.0 (9b00956e5 2024-04-29)")
///   6. extra_flags (profile, features, etc. — caller-supplied sorted map)
pub fn compute_hash(
    recipe_name: &str,
    recipe_type: &str,
    input_files: &[&Path],
    env: &BTreeMap<String, String>,
    toolchain_id: &str,
    extra_flags: &BTreeMap<String, String>,
) -> String {
    let mut hasher = Sha256::new();

    hasher.update(recipe_name.as_bytes());
    hasher.update(b"\x00");
    hasher.update(recipe_type.as_bytes());
    hasher.update(b"\x00");

    // Hash input files in sorted path order for determinism
    let mut sorted_inputs: Vec<&Path> = input_files.to_vec();
    sorted_inputs.sort();
    for path in sorted_inputs {
        hasher.update(path.to_string_lossy().as_bytes());
        hasher.update(b"\x00");
        match std::fs::read(path) {
            Ok(contents) => {
                let mut file_hasher = Sha256::new();
                file_hasher.update(&contents);
                hasher.update(file_hasher.finalize());
            }
            Err(_) => {
                // Missing input — mark as absent so the hash differs from "empty file"
                hasher.update(b"<missing>\x00");
            }
        }
    }

    // Env vars (sorted BTreeMap guarantees order)
    for (k, v) in env {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b"\x00");
    }

    hasher.update(toolchain_id.as_bytes());
    hasher.update(b"\x00");

    for (k, v) in extra_flags {
        hasher.update(k.as_bytes());
        hasher.update(b"=");
        hasher.update(v.as_bytes());
        hasher.update(b"\x00");
    }

    hex::encode(hasher.finalize())
}

/// Hash a single file's contents. Returns a hex string, or a sentinel for missing files.
pub fn hash_file(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(contents) => {
            let mut hasher = Sha256::new();
            hasher.update(&contents);
            hex::encode(hasher.finalize())
        }
        Err(_) => "<missing>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn empty_env() -> BTreeMap<String, String> {
        BTreeMap::new()
    }
    fn empty_flags() -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    #[test]
    fn test_same_inputs_same_hash() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello").unwrap();
        let h1 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.78",
            &empty_flags(),
        );
        let h2 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.78",
            &empty_flags(),
        );
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_changed_content_changes_hash() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello").unwrap();
        let h1 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.78",
            &empty_flags(),
        );
        std::fs::write(&f, "world").unwrap();
        let h2 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.78",
            &empty_flags(),
        );
        assert_ne!(h1, h2, "changing file content must change the hash");
    }

    #[test]
    fn test_changed_env_changes_hash() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello").unwrap();
        let mut env = BTreeMap::new();
        env.insert("PROFILE".to_string(), "debug".to_string());
        let h1 = compute_hash("build", "shell", &[&f], &env, "rustc 1.78", &empty_flags());
        env.insert("PROFILE".to_string(), "release".to_string());
        let h2 = compute_hash("build", "shell", &[&f], &env, "rustc 1.78", &empty_flags());
        assert_ne!(h1, h2, "changing env must change the hash");
    }

    #[test]
    fn test_changed_toolchain_changes_hash() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("a.txt");
        std::fs::write(&f, "hello").unwrap();
        let h1 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.78.0",
            &empty_flags(),
        );
        let h2 = compute_hash(
            "build",
            "shell",
            &[&f],
            &empty_env(),
            "rustc 1.79.0",
            &empty_flags(),
        );
        assert_ne!(h1, h2, "changing toolchain must change the hash");
    }

    #[test]
    fn test_input_order_does_not_matter() {
        let dir = tempfile::TempDir::new().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, "aaa").unwrap();
        std::fs::write(&b, "bbb").unwrap();
        let h1 = compute_hash(
            "build",
            "shell",
            &[&a, &b],
            &empty_env(),
            "tc",
            &empty_flags(),
        );
        let h2 = compute_hash(
            "build",
            "shell",
            &[&b, &a],
            &empty_env(),
            "tc",
            &empty_flags(),
        );
        assert_eq!(h1, h2, "input order must not affect hash");
    }

    #[test]
    fn test_hash_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let f = dir.path().join("f.txt");
        std::fs::write(&f, "content").unwrap();
        let h = hash_file(&f);
        assert_eq!(h.len(), 64, "SHA-256 hex is 64 chars");
        assert_ne!(hash_file(std::path::Path::new("/nonexistent")), h);
    }
}
