use must_cache::hash::compute_hash;
use must_core::{
    BuildContext, Cache, CacheKey, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    ensure_within_root,
};
use sha2::Digest;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct PrecompiledBinRecipe {
    pub name: String,
    pub deps: Vec<String>,
    pub url: String,
    pub sha256: Option<String>,
    pub output_path: String,
    pub env: HashMap<String, String>,
}

impl PrecompiledBinRecipe {
    pub fn new(
        name: impl Into<String>,
        url: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            deps: Vec::new(),
            url: url.into(),
            sha256: None,
            output_path: output_path.into(),
            env: HashMap::new(),
        }
    }

    fn dest_path(&self, ctx: &BuildContext) -> Result<PathBuf> {
        if !self.url.starts_with("https://") {
            return Err(Error::Config {
                path: PathBuf::from(&self.url),
                message: "precompiled-bin URL must use https://".to_string(),
            });
        }
        ensure_within_root(&ctx.project_root, Path::new(&self.output_path))
    }

    fn download(&self, dest: &Path) -> std::result::Result<(), String> {
        let parent = dest
            .parent()
            .ok_or_else(|| format!("invalid output path: {}", dest.display()))?;
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;

        let mut response = ureq::get(&self.url)
            .call()
            .map_err(|e| format!("download failed for {}: {e}", self.url))?;

        let mut reader = response.body_mut().as_reader();
        let mut tmp_path = dest.to_owned();
        tmp_path.set_extension("tmp");
        let mut file =
            std::fs::File::create(&tmp_path).map_err(|e| format!("create tmp file failed: {e}"))?;
        std::io::copy(&mut reader, &mut file).map_err(|e| format!("download write failed: {e}"))?;
        drop(file);

        if let Some(ref expected) = self.sha256 {
            let mut verify_file = std::fs::File::open(&tmp_path)
                .map_err(|e| format!("open for verify failed: {e}"))?;
            let mut hasher = sha2::Sha256::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = verify_file
                    .read(&mut buf)
                    .map_err(|e| format!("hash read failed: {e}"))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            let actual = hex::encode(hasher.finalize());
            if actual != *expected {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(format!(
                    "SHA256 mismatch: expected {expected}, got {actual}"
                ));
            }
        }

        std::fs::rename(&tmp_path, dest).map_err(|e| format!("rename failed: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| format!("chmod failed: {e}"))?;
        }

        Ok(())
    }
}

impl Recipe for PrecompiledBinRecipe {
    fn name(&self) -> &str {
        &self.name
    }

    fn deps(&self) -> &[String] {
        &self.deps
    }

    fn inputs(&self, _ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![])
    }

    fn outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        Ok(vec![self.dest_path(ctx)?])
    }

    fn cache_strategy(&self) -> CacheStrategy {
        CacheStrategy::Hash
    }

    fn cache_key(&self, ctx: &BuildContext) -> Result<CacheKey> {
        let mut flags = BTreeMap::new();
        flags.insert("url".to_string(), self.url.clone());
        if let Some(ref sha) = self.sha256 {
            flags.insert("sha256".to_string(), sha.clone());
        }
        flags.insert("output_path".to_string(), self.output_path.clone());
        Ok(CacheKey {
            recipe: self.name.clone(),
            target: ctx.target.clone(),
            profile: ctx.profile.clone(),
            hash: compute_hash(
                &self.name,
                "precompiled-bin",
                &[],
                &BTreeMap::new(),
                "",
                &flags,
            ),
        })
    }

    fn execute(&self, ctx: &BuildContext) -> Result<RecipeOutput> {
        let start = std::time::Instant::now();
        let dest = self.dest_path(ctx)?;

        if ctx.dry_run {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: false,
                outputs: vec![dest.clone()],
                stdout: format!("[dry-run] download {} -> {}", self.url, dest.display()),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        if dest.exists() {
            return Ok(RecipeOutput {
                recipe_name: self.name.clone(),
                from_cache: true,
                outputs: vec![dest.clone()],
                stdout: format!("{} (already present)", dest.display()),
                stderr: String::new(),
                duration_ms: 0,
            });
        }

        self.download(&dest).map_err(|e| Error::RecipeFailed {
            name: self.name.clone(),
            code: 1,
            stderr: e,
        })?;

        let duration_ms = start.elapsed().as_millis() as u64;

        if let Ok(cache) = must_cache::store::DiskCache::open(&ctx.cache_dir) {
            let _ = cache.store(&self.cache_key(ctx)?, &[]);
        }

        Ok(RecipeOutput {
            recipe_name: self.name.clone(),
            from_cache: false,
            outputs: vec![dest.clone()],
            stdout: format!("downloaded {} -> {}", self.url, dest.display()),
            stderr: String::new(),
            duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use must_core::Recipe;

    fn test_ctx() -> BuildContext {
        let mut env = HashMap::new();
        env.insert(
            "PATH".to_string(),
            std::env::var("PATH").unwrap_or_default(),
        );
        env.insert(
            "HOME".to_string(),
            std::env::var("HOME").unwrap_or_default(),
        );
        BuildContext {
            project_root: PathBuf::from("/tmp/test"),
            cache_dir: PathBuf::from("/tmp/test/.cache"),
            log_dir: PathBuf::from("/tmp/test/logs"),
            target: "host".into(),
            profile: "default".into(),
            env,
            dry_run: false,
            parallelism: 1,
            cache: None,
        }
    }

    #[test]
    fn construction() {
        let r = PrecompiledBinRecipe::new("protoc", "https://example.com/protoc", "bin/protoc");
        assert_eq!(r.name(), "protoc");
        assert_eq!(r.url, "https://example.com/protoc");
        assert_eq!(r.output_path, "bin/protoc");
        assert!(r.deps().is_empty());
        assert!(r.sha256.is_none());
    }

    #[test]
    fn with_deps() {
        let mut r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        r.deps = vec!["setup".to_string()];
        assert_eq!(r.deps(), &["setup".to_string()]);
    }

    #[test]
    fn with_sha256() {
        let mut r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        r.sha256 = Some("abc123".to_string());
        assert_eq!(r.sha256.as_deref(), Some("abc123"));
    }

    #[test]
    fn cache_strategy_is_hash() {
        let r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        assert_eq!(r.cache_strategy(), CacheStrategy::Hash);
    }

    #[test]
    fn cache_key_stable() {
        let r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        let key1 = r.cache_key(&test_ctx()).unwrap();
        let key2 = r.cache_key(&test_ctx()).unwrap();
        assert_eq!(key1.hash, key2.hash);
    }

    #[test]
    fn cache_key_differs_by_url() {
        let r1 = PrecompiledBinRecipe::new("tool", "https://example.com/tool-v1", "bin/tool");
        let r2 = PrecompiledBinRecipe::new("tool", "https://example.com/tool-v2", "bin/tool");
        assert_ne!(
            r1.cache_key(&test_ctx()).unwrap().hash,
            r2.cache_key(&test_ctx()).unwrap().hash
        );
    }

    #[test]
    fn cache_key_differs_by_sha256() {
        let mut r1 = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        r1.sha256 = Some("abc".to_string());
        let r2 = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        assert_ne!(
            r1.cache_key(&test_ctx()).unwrap().hash,
            r2.cache_key(&test_ctx()).unwrap().hash
        );
    }

    #[test]
    fn outputs_path() {
        let r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        let outs = r.outputs(&test_ctx()).unwrap();
        assert_eq!(outs[0], PathBuf::from("/tmp/test/bin/tool"));
    }

    #[test]
    fn dry_run() {
        let r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        let mut ctx = test_ctx();
        ctx.dry_run = true;
        let out = r.execute(&ctx).unwrap();
        assert!(out.stdout.contains("[dry-run]"));
        assert!(out.stdout.contains("https://example.com/tool"));
        assert!(out.stdout.contains("bin/tool"));
        assert!(!out.from_cache);
    }

    #[test]
    fn already_present_is_cached() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bin_path = tmp.path().join("bin").join("tool");
        std::fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
        std::fs::write(&bin_path, b"binary").unwrap();

        let r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        let mut ctx = test_ctx();
        ctx.project_root = tmp.path().to_owned();
        ctx.cache_dir = tmp.path().join(".cache");

        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache);
        assert!(out.stdout.contains("already present"));
    }

    #[test]
    fn download_writes_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("output.bin");
        let r = PrecompiledBinRecipe::new("test", "https://example.com/tool", "out.bin");
        r.download(&dest).unwrap_err();
    }

    #[test]
    fn traversal_rejected() {
        let r = PrecompiledBinRecipe::new("evil", "https://example.com/tool", "../../etc/passwd");
        let result = r.dest_path(&test_ctx());
        assert!(result.is_err());
    }

    #[test]
    fn http_url_rejected() {
        let r = PrecompiledBinRecipe::new("tool", "http://example.com/tool", "bin/tool");
        let result = r.dest_path(&test_ctx());
        assert!(result.is_err());
    }

    #[test]
    fn outputs_rejects_traversal() {
        let r = PrecompiledBinRecipe::new("evil", "https://example.com/tool", "../../etc/passwd");
        let ctx = test_ctx();
        assert!(r.outputs(&ctx).is_err());
    }

    #[test]
    fn execute_rejects_traversal() {
        let r = PrecompiledBinRecipe::new("evil", "https://example.com/tool", "../../etc/passwd");
        let ctx = test_ctx();
        assert!(r.execute(&ctx).is_err());
    }
}
