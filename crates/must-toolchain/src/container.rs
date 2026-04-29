use crate::triple::Triple;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The container runtime to use for cross-compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerRuntime {
    Docker,
    Podman,
}

impl ContainerRuntime {
    /// Returns the binary name for this runtime.
    pub fn binary(&self) -> &str {
        match self {
            ContainerRuntime::Docker => "docker",
            ContainerRuntime::Podman => "podman",
        }
    }
}

/// Detect which container runtime is available (docker first, then podman).
/// Returns `None` if neither is available.
pub fn detect_runtime() -> Option<ContainerRuntime> {
    if Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some(ContainerRuntime::Docker);
    }

    if Command::new("podman")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some(ContainerRuntime::Podman);
    }

    None
}

/// Returns `true` if any container runtime is available.
pub fn container_available() -> bool {
    detect_runtime().is_some()
}

/// Map a target triple + recipe type to a dockcross container image.
/// Returns `None` if no image is known for this combination.
pub fn image_for(triple: &Triple, recipe_type: &str) -> Option<String> {
    let image = match (triple.raw.as_str(), recipe_type) {
        ("x86_64-unknown-linux-gnu", "c-bin") | ("x86_64-unknown-linux-gnu", "c-lib") => {
            "dockcross/linux-x64"
        }
        ("x86_64-unknown-linux-musl", _) => "dockcross/linux-x64-musl",
        ("aarch64-unknown-linux-gnu", "c-bin") | ("aarch64-unknown-linux-gnu", "c-lib") => {
            "dockcross/linux-arm64"
        }
        ("aarch64-unknown-linux-musl", _) => "dockcross/linux-arm64-musl",
        ("x86_64-unknown-linux-gnu", "rust-bin")
        | ("x86_64-unknown-linux-gnu", "rust-lib")
        | ("aarch64-unknown-linux-gnu", "rust-bin")
        | ("aarch64-unknown-linux-gnu", "rust-lib") => "ghcr.io/cross-rs/cross:latest",
        _ => return None,
    };

    Some(image.to_string())
}

/// A toolchain that runs build commands inside a container.
pub struct ContainerToolchain {
    pub triple: Triple,
    pub image: String,
    pub runtime: ContainerRuntime,
    /// Host-side project root.
    pub project_root: PathBuf,
    /// Mount path inside the container, always `/work`.
    pub mount_path: PathBuf,
}

impl ContainerToolchain {
    /// Build a `ContainerToolchain`, choosing the image automatically if not provided.
    ///
    /// Returns `Err` if no container runtime is detected, or if no image is known for
    /// this triple + recipe_type combination and no `image_override` was given.
    pub fn new(
        triple: Triple,
        recipe_type: &str,
        project_root: PathBuf,
        image_override: Option<String>,
    ) -> Result<Self, String> {
        let runtime = detect_runtime()
            .ok_or_else(|| "no container runtime detected (docker or podman required)".to_string())?;

        let image = match image_override {
            Some(img) => img,
            None => image_for(&triple, recipe_type).ok_or_else(|| {
                format!(
                    "no container image known for triple '{}' with recipe type '{}'",
                    triple.raw, recipe_type
                )
            })?,
        };

        Ok(ContainerToolchain {
            triple,
            image,
            runtime,
            project_root,
            mount_path: PathBuf::from("/work"),
        })
    }

    /// Build the `docker run` / `podman run` command that wraps a build command.
    ///
    /// The generated command:
    ///   `<runtime> run --rm -v <project_root>:/work -w /work <image> <cmd> <args...>`
    pub fn wrap_command(&self, cmd: &str, args: &[&str]) -> Command {
        let mut command = Command::new(self.runtime.binary());
        command
            .arg("run")
            .arg("--rm")
            .arg("-v")
            .arg(format!("{}:/work", self.project_root.display()))
            .arg("-w")
            .arg("/work")
            .arg(&self.image)
            .arg(cmd)
            .args(args);
        command
    }

    /// Translate a host-side path under `project_root` to the container-side equivalent.
    ///
    /// For example: `/home/user/myproject/src/main.c` → `/work/src/main.c`
    ///
    /// If `host_path` does not start with `project_root`, it is returned unchanged.
    pub fn translate_path(&self, host_path: &Path) -> PathBuf {
        if let Ok(relative) = host_path.strip_prefix(&self.project_root) {
            self.mount_path.join(relative)
        } else {
            host_path.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triple::Triple;

    #[test]
    fn test_image_for_aarch64_c_bin() {
        let triple = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(
            image_for(&triple, "c-bin"),
            Some("dockcross/linux-arm64".to_string())
        );
    }

    #[test]
    fn test_image_for_unknown_triple() {
        let triple = Triple::parse("x86_64-apple-darwin").unwrap();
        assert_eq!(image_for(&triple, "c-bin"), None);
    }

    #[test]
    fn test_container_available_does_not_panic() {
        // Just call it — we don't assert a specific value since CI may or may not have Docker.
        let _ = container_available();
    }

    #[test]
    fn test_translate_path_under_project_root() {
        let tc = ContainerToolchain {
            triple: Triple::parse("aarch64-unknown-linux-gnu").unwrap(),
            image: "dockcross/linux-arm64".to_string(),
            runtime: ContainerRuntime::Docker,
            project_root: PathBuf::from("/home/user/proj"),
            mount_path: PathBuf::from("/work"),
        };
        let result = tc.translate_path(Path::new("/home/user/proj/src/main.c"));
        assert_eq!(result, PathBuf::from("/work/src/main.c"));
    }

    #[test]
    fn test_translate_path_outside_project_root() {
        let tc = ContainerToolchain {
            triple: Triple::parse("aarch64-unknown-linux-gnu").unwrap(),
            image: "dockcross/linux-arm64".to_string(),
            runtime: ContainerRuntime::Docker,
            project_root: PathBuf::from("/home/user/proj"),
            mount_path: PathBuf::from("/work"),
        };
        let outside = Path::new("/tmp/other/file.c");
        let result = tc.translate_path(outside);
        assert_eq!(result, PathBuf::from("/tmp/other/file.c"));
    }

    #[test]
    fn test_container_runtime_binary_names() {
        assert_eq!(ContainerRuntime::Docker.binary(), "docker");
        assert_eq!(ContainerRuntime::Podman.binary(), "podman");
    }

    #[test]
    fn test_image_for_x86_64_c_bin() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(image_for(&triple, "c-bin"), Some("dockcross/linux-x64".to_string()));
    }

    #[test]
    fn test_image_for_x86_64_c_lib() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(image_for(&triple, "c-lib"), Some("dockcross/linux-x64".to_string()));
    }

    #[test]
    fn test_image_for_x86_64_musl() {
        let triple = Triple::parse("x86_64-unknown-linux-musl").unwrap();
        assert_eq!(image_for(&triple, "c-bin"), Some("dockcross/linux-x64-musl".to_string()));
        assert_eq!(image_for(&triple, "rust-bin"), Some("dockcross/linux-x64-musl".to_string()));
    }

    #[test]
    fn test_image_for_aarch64_musl() {
        let triple = Triple::parse("aarch64-unknown-linux-musl").unwrap();
        assert_eq!(image_for(&triple, "c-lib"), Some("dockcross/linux-arm64-musl".to_string()));
    }

    #[test]
    fn test_image_for_rust_cross_compilation() {
        let triple = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(image_for(&triple, "rust-bin"), Some("ghcr.io/cross-rs/cross:latest".to_string()));
        assert_eq!(image_for(&triple, "rust-lib"), Some("ghcr.io/cross-rs/cross:latest".to_string()));
        let triple2 = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(image_for(&triple2, "rust-bin"), Some("ghcr.io/cross-rs/cross:latest".to_string()));
    }

    #[test]
    fn test_container_toolchain_new_no_image_returns_err() {
        // x86_64-apple-darwin has no known container image → Err (regardless of runtime)
        let triple = Triple::parse("x86_64-apple-darwin").unwrap();
        let result = ContainerToolchain::new(triple, "c-bin", std::path::PathBuf::from("/tmp"), None);
        assert!(result.is_err(), "should fail when no runtime or no image");
    }

    #[test]
    fn test_wrap_command_structure() {
        let tc = ContainerToolchain {
            triple: Triple::parse("aarch64-unknown-linux-gnu").unwrap(),
            image: "dockcross/linux-arm64".to_string(),
            runtime: ContainerRuntime::Docker,
            project_root: PathBuf::from("/home/user/proj"),
            mount_path: PathBuf::from("/work"),
        };
        let cmd = tc.wrap_command("cc", &["-o", "out", "main.c"]);
        // Program should be "docker"
        assert_eq!(cmd.get_program(), "docker");
        // Collect args as strings
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"--rm".to_string()), "args should contain --rm: {:?}", args);
        assert!(args.contains(&"-v".to_string()), "args should contain -v: {:?}", args);
        assert!(
            args.iter().any(|a| a.contains("/work")),
            "args should contain /work: {:?}",
            args
        );
    }
}
