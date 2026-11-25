use std::path::{Path, PathBuf};

use crate::Error;

pub fn ensure_within_root(root: &Path, path: &Path) -> crate::Result<PathBuf> {
    for component in path.components() {
        match component {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err(Error::Config {
                    path: path.to_owned(),
                    message: "path must not contain '..' components".to_string(),
                });
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(Error::Config {
                    path: path.to_owned(),
                    message: "path must be relative to the project root".to_string(),
                });
            }
        }
    }
    Ok(root.join(path))
}

pub fn validate_name_no_traversal(name: &str) -> crate::Result<()> {
    let mut components = Path::new(name).components();
    let is_plain = matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
        && !name.contains("..")
        && !name.contains('/')
        && !name.contains('\\');
    if is_plain {
        Ok(())
    } else {
        Err(Error::Config {
            path: PathBuf::from(name),
            message: "name must not contain '..' or path separators".to_string(),
        })
    }
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
    fn absolute_path_rejected() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("/etc/cron.d/backdoor"));
        assert!(result.is_err());
    }

    #[test]
    fn rooted_relative_path_rejected() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("\\windows\\system32\\evil.dll"));
        if cfg!(windows) {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok());
        }
    }

    #[test]
    fn dot_component_passes() {
        let root = PathBuf::from("/tmp/project");
        let result = ensure_within_root(&root, Path::new("./bin/tool"));
        assert!(result.is_ok());
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
        assert!(validate_name_no_traversal("foo\\bar").is_err());
        assert!(validate_name_no_traversal("foo/").is_err());
    }

    #[test]
    fn empty_and_dot_names_rejected() {
        assert!(validate_name_no_traversal("").is_err());
        assert!(validate_name_no_traversal(".").is_err());
        assert!(validate_name_no_traversal("./build").is_err());
    }

    #[test]
    fn rooted_and_prefixed_names_rejected() {
        assert!(validate_name_no_traversal("/tmp/evil").is_err());
        assert!(validate_name_no_traversal("\\\\server\\share").is_err());
    }

    #[test]
    #[cfg(windows)]
    fn drive_relative_name_rejected_on_windows() {
        assert!(validate_name_no_traversal("C:evil").is_err());
    }
}
