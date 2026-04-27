use must_core::{Error, Result};

/// Parsed architecture component of a target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Riscv64,
}

/// Parsed OS component of a target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
    Windows,
}

/// A parsed, validated target triple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Triple {
    pub arch: Arch,
    pub vendor: String,
    pub os: Os,
    pub env: Option<String>,
    /// The original string, e.g. "aarch64-unknown-linux-gnu", or "host".
    pub raw: String,
}

impl Triple {
    /// Parse a target triple string. Returns `Err` if arch or OS is unrecognized.
    ///
    /// Also accepts `"host"` — delegates to [`Triple::host`].
    pub fn parse(s: &str) -> Result<Self> {
        if s == "host" {
            return Ok(Self::host());
        }

        let parts: Vec<&str> = s.splitn(4, '-').collect();
        if parts.len() < 3 {
            return Err(Error::ToolchainNotFound {
                target: s.to_string(),
                hint: format!("'{}' is not a recognized target triple", s),
            });
        }

        let arch_str = parts[0];
        let vendor = parts[1].to_string();
        let os_str = parts[2];
        let env = parts.get(3).map(|e| e.to_string());

        let arch = match arch_str {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            "riscv64" => Arch::Riscv64,
            other => {
                return Err(Error::ToolchainNotFound {
                    target: s.to_string(),
                    hint: format!(
                        "unrecognized architecture '{}'; supported: x86_64, aarch64, riscv64",
                        other
                    ),
                })
            }
        };

        // For x86_64-pc-windows-msvc, splitn(4,'-') gives:
        //   parts[0]="x86_64", parts[1]="pc", parts[2]="windows", parts[3]="msvc"
        // so os_str is already "windows". The "apple" vendor is used for macOS triples
        // like x86_64-apple-darwin where os_str is "darwin".
        let os = match os_str {
            "linux" => Os::Linux,
            "apple" => Os::Macos,
            "darwin" => Os::Macos,
            "windows" => Os::Windows,
            other => {
                return Err(Error::ToolchainNotFound {
                    target: s.to_string(),
                    hint: format!(
                        "unrecognized OS '{}'; supported: linux, apple/darwin, windows",
                        other
                    ),
                })
            }
        };

        Ok(Triple {
            arch,
            vendor,
            os,
            env,
            raw: s.to_string(),
        })
    }

    /// Return a `Triple` representing the host platform.
    ///
    /// Detects arch and OS via `std::env::consts`.
    pub fn host() -> Self {
        let arch = match std::env::consts::ARCH {
            "x86_64" => Arch::X86_64,
            "aarch64" => Arch::Aarch64,
            "riscv64" => Arch::Riscv64,
            // Fallback to X86_64 for unknown host architectures.
            _ => Arch::X86_64,
        };

        let os = match std::env::consts::OS {
            "linux" => Os::Linux,
            "macos" => Os::Macos,
            "windows" => Os::Windows,
            // Fallback to Linux for unknown host OS.
            _ => Os::Linux,
        };

        Triple {
            arch,
            vendor: "unknown".to_string(),
            os,
            env: None,
            raw: "host".to_string(),
        }
    }

    /// Returns `true` if this triple represents the host (sentinel raw value).
    pub fn is_host(&self) -> bool {
        self.raw == "host"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_x86_64_linux_gnu() {
        let t = Triple::parse("x86_64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.vendor, "unknown");
        assert_eq!(t.os, Os::Linux);
        assert_eq!(t.env, Some("gnu".to_string()));
        assert_eq!(t.raw, "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn test_parse_x86_64_linux_musl() {
        let t = Triple::parse("x86_64-unknown-linux-musl").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.os, Os::Linux);
        assert_eq!(t.env, Some("musl".to_string()));
    }

    #[test]
    fn test_parse_aarch64_linux_gnu() {
        let t = Triple::parse("aarch64-unknown-linux-gnu").unwrap();
        assert_eq!(t.arch, Arch::Aarch64);
        assert_eq!(t.os, Os::Linux);
        assert_eq!(t.env, Some("gnu".to_string()));
    }

    #[test]
    fn test_parse_aarch64_linux_musl() {
        let t = Triple::parse("aarch64-unknown-linux-musl").unwrap();
        assert_eq!(t.arch, Arch::Aarch64);
        assert_eq!(t.os, Os::Linux);
        assert_eq!(t.env, Some("musl".to_string()));
    }

    #[test]
    fn test_parse_x86_64_apple_darwin() {
        let t = Triple::parse("x86_64-apple-darwin").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.vendor, "apple");
        assert_eq!(t.os, Os::Macos);
        assert_eq!(t.env, None);
    }

    #[test]
    fn test_parse_aarch64_apple_darwin() {
        let t = Triple::parse("aarch64-apple-darwin").unwrap();
        assert_eq!(t.arch, Arch::Aarch64);
        assert_eq!(t.os, Os::Macos);
        assert_eq!(t.env, None);
    }

    #[test]
    fn test_parse_x86_64_windows_msvc() {
        let t = Triple::parse("x86_64-pc-windows-msvc").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.os, Os::Windows);
        assert_eq!(t.env, Some("msvc".to_string()));
    }

    #[test]
    fn test_parse_x86_64_windows_gnu() {
        let t = Triple::parse("x86_64-pc-windows-gnu").unwrap();
        assert_eq!(t.arch, Arch::X86_64);
        assert_eq!(t.os, Os::Windows);
        assert_eq!(t.env, Some("gnu".to_string()));
    }

    #[test]
    fn test_parse_host() {
        let t = Triple::parse("host").unwrap();
        assert!(t.is_host());
        assert_eq!(t.raw, "host");
    }

    #[test]
    fn test_host_constructor() {
        let t = Triple::host();
        assert!(t.is_host());
    }

    #[test]
    fn test_parse_rejects_unknown_arch() {
        let result = Triple::parse("haskell-unknown-linux-gnu");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("haskell-unknown-linux-gnu") || err.contains("haskell"));
    }

    #[test]
    fn test_parse_rejects_too_few_parts() {
        assert!(Triple::parse("x86_64-linux").is_err());
    }
}
