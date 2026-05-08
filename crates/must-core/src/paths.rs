use std::path::{Path, PathBuf};

use crate::Error;

pub fn ensure_within_root(root: &Path, path: &Path) -> crate::Result<PathBuf> {
    let resolved = root.join(path);
    let has_traversal = path.components().any(|c| {
        matches!(c, std::path::Component::ParentDir)
    });
    if has_traversal {
        return Err(Error::Config {
            path: path.to_owned(),
            message: "path must not contain '..' components".to_string(),
        });
    }
    Ok(resolved)
}

pub fn validate_name_no_traversal(name: &str) -> crate::Result<()> {
    if name.contains("..") || name.contains(std::path::MAIN_SEPARATOR) {
        return Err(Error::Config {
            path: PathBuf::from(name),
            message: "name must not contain '..' or path separators".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normal_path_passes() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("bin/tool"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/tmp/project/bin/tool"));
    }

    #[test]
    fn traversal_rejected() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("../../etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn dot_dot_in_middle_rejected() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("foo/../../etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn simple_name_passes() {
        assert!(validate_name_no_traversal("build").is_ok());
        assert!(validate_name_no_traversal("my-recipe").is_ok());
    }

    #[test]
    fn name_with_dot_dot_rejected() {
        assert!(validate_name_no_traversal("../evil").is_err());
    }

    #[test]
    fn name_with_separator_rejected() {
        assert!(validate_name_no_traversal("foo/bar").is_err());
    }
}
