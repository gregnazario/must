use must_cache::hash::{compute_hash, hash_file};
use must_core::{
    BuildContext, Cache, CacheKey, CacheStrategy, Error, Recipe, RecipeOutput, Result,
    ensure_within_root,
};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

fn dest_matches_pinned_sha256(dest: &Path, expected: Option<&str>) -> bool {
    match expected {
        None => true,
        Some(expected) => hash_file(dest) == expected,
    }
}

fn archive_kind(url: &str) -> Option<&'static str> {
    let lower = url.to_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        Some("tar.gz")
    } else if lower.ends_with(".zip") {
        Some("zip")
    } else {
        None
    }
}

fn sha_marker_path(dest: &Path) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".mustsha256");
    dest.with_file_name(name)
}

fn installed_matches_pin(url: &str, sha256: Option<&str>, dest: &Path) -> bool {
    match sha256 {
        None => true,
        Some(expected) => {
            if archive_kind(url).is_some() {
                std::fs::read_to_string(sha_marker_path(dest))
                    .map(|m| m.trim() == expected)
                    .unwrap_or(false)
            } else {
                dest_matches_pinned_sha256(dest, Some(expected))
            }
        }
    }
}

fn extract_archive(archive: &Path, kind: &str, out_dir: &Path) -> std::result::Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| format!("mkdir failed: {e}"))?;
    match kind {
        "tar.gz" => {
            let file =
                std::fs::File::open(archive).map_err(|e| format!("open archive failed: {e}"))?;
            let gz = flate2::read::GzDecoder::new(file);
            tar::Archive::new(gz)
                .unpack(out_dir)
                .map_err(|e| format!("extract tar.gz failed: {e}"))
        }
        "zip" => {
            let file =
                std::fs::File::open(archive).map_err(|e| format!("open archive failed: {e}"))?;
            let mut zip =
                zip::ZipArchive::new(file).map_err(|e| format!("open zip failed: {e}"))?;
            for i in 0..zip.len() {
                let mut entry = zip
                    .by_index(i)
                    .map_err(|e| format!("read zip entry failed: {e}"))?;
                let Some(rel) = entry.enclosed_name() else {
                    continue;
                };
                let dest = out_dir.join(rel);
                if entry.is_dir() {
                    std::fs::create_dir_all(&dest).map_err(|e| format!("mkdir failed: {e}"))?;
                } else {
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| format!("mkdir failed: {e}"))?;
                    }
                    let mut out = std::fs::File::create(&dest)
                        .map_err(|e| format!("create file failed: {e}"))?;
                    std::io::copy(&mut entry, &mut out)
                        .map_err(|e| format!("extract zip entry failed: {e}"))?;
                    #[cfg(unix)]
                    if let Some(mode) = entry.unix_mode() {
                        use std::os::unix::fs::PermissionsExt;
                        let _ =
                            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(mode));
                    }
                }
            }
            Ok(())
        }
        _ => Err(format!("unknown archive kind: {kind}")),
    }
}

fn is_executable_file(path: &Path) -> bool {
    if cfg!(windows) {
        path.extension().is_some_and(|e| e == "exe")
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(path)
                .map(|m| m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            false
        }
    }
}

fn is_doc_artifact(path: &Path) -> bool {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let doc_prefixes = [
        "readme",
        "license",
        "copying",
        "changelog",
        "notice",
        "authors",
    ];
    let doc_exts = [".md", ".txt", ".1", ".html", ".proto"];
    doc_prefixes.iter().any(|p| name.starts_with(p)) || doc_exts.iter().any(|e| name.ends_with(e))
}

fn pick_binary(extract_dir: &Path, recipe_name: &str) -> std::result::Result<PathBuf, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![extract_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("read dir failed: {e}"))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();

    if let Some(exact) = files
        .iter()
        .find(|p| p.file_name().is_some_and(|n| n == recipe_name))
    {
        return Ok(exact.clone());
    }

    let executables: Vec<&PathBuf> = files.iter().filter(|p| is_executable_file(p)).collect();
    let candidates: Vec<&PathBuf> = if executables.is_empty() {
        files.iter().filter(|p| !is_doc_artifact(p)).collect()
    } else {
        executables
    };

    match candidates.len() {
        1 => Ok(candidates[0].clone()),
        0 => Err("archive contains no candidate binary".to_string()),
        _ => {
            let listing: Vec<String> = candidates
                .iter()
                .map(|p| {
                    p.strip_prefix(extract_dir)
                        .unwrap_or(p)
                        .display()
                        .to_string()
                })
                .collect();
            Err(format!(
                "archive contains multiple candidate binaries: [{}]; name the recipe after the binary or use a raw file URL",
                listing.join(", ")
            ))
        }
    }
}

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

        let agent: ureq::Agent = ureq::Agent::config_builder()
            .https_only(true)
            .build()
            .into();
        let mut response = agent
            .get(&self.url)
            .call()
            .map_err(|e| format!("download failed for {}: {e}", self.url))?;

        let mut reader = response.body_mut().as_reader();
        let mut tmp_path = dest.to_owned();
        tmp_path.set_extension("tmp");
        let mut file =
            std::fs::File::create(&tmp_path).map_err(|e| format!("create tmp file failed: {e}"))?;
        if let Err(e) = std::io::copy(&mut reader, &mut file) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(format!("download write failed: {e}"));
        }
        drop(file);

        if let Some(ref expected) = self.sha256 {
            let actual = hash_file(&tmp_path);
            if actual != *expected {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(format!(
                    "SHA256 mismatch: expected {expected}, got {actual}"
                ));
            }
        }

        if let Some(kind) = archive_kind(&self.url) {
            let mut extract_dir = dest.to_owned();
            extract_dir.set_extension("extract");
            if let Err(e) = extract_archive(&tmp_path, kind, &extract_dir) {
                let _ = std::fs::remove_file(&tmp_path);
                let _ = std::fs::remove_dir_all(&extract_dir);
                return Err(e);
            }
            let binary = pick_binary(&extract_dir, &self.name);
            let install = binary.and_then(|bin| {
                let _ = std::fs::remove_file(dest);
                std::fs::copy(&bin, dest).map_err(|e| format!("install failed: {e}"))
            });
            let _ = std::fs::remove_file(&tmp_path);
            let _ = std::fs::remove_dir_all(&extract_dir);
            install?;

            if let Some(ref expected) = self.sha256 {
                std::fs::write(sha_marker_path(dest), expected)
                    .map_err(|e| format!("write sha marker failed: {e}"))?;
            }
        } else {
            let _ = std::fs::remove_file(dest);
            std::fs::rename(&tmp_path, dest).map_err(|e| format!("rename failed: {e}"))?;
            let _ = std::fs::remove_file(sha_marker_path(dest));
        }

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

        if dest.exists() && installed_matches_pin(&self.url, self.sha256.as_deref(), &dest) {
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
            let _ = cache.store(&self.cache_key(ctx)?, &ctx.project_root, &[]);
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
    fn already_present_with_matching_sha256_is_cached() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bin_path = tmp.path().join("bin").join("tool");
        std::fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
        std::fs::write(&bin_path, b"binary").unwrap();

        let mut r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        r.sha256 = Some(hash_file(&bin_path));
        let mut ctx = test_ctx();
        ctx.project_root = tmp.path().to_owned();
        ctx.cache_dir = tmp.path().join(".cache");

        let out = r.execute(&ctx).unwrap();
        assert!(out.from_cache);
        assert!(out.stdout.contains("already present"));
    }

    #[test]
    fn already_present_with_wrong_sha256_triggers_redownload() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bin_path = tmp.path().join("bin").join("tool");
        std::fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
        std::fs::write(&bin_path, b"stale-content").unwrap();

        let mut r = PrecompiledBinRecipe::new("tool", "https://example.com/tool", "bin/tool");
        r.sha256 = Some("deadbeef".to_string());
        let mut ctx = test_ctx();
        ctx.project_root = tmp.path().to_owned();
        ctx.cache_dir = tmp.path().join(".cache");

        match r.execute(&ctx) {
            Ok(out) => panic!(
                "expected re-download attempt, got cached output: {}",
                out.stdout
            ),
            Err(Error::RecipeFailed { .. }) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn dest_matches_pinned_sha256_checks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let p = tmp.path().join("tool");
        std::fs::write(&p, b"binary").unwrap();
        let actual = hash_file(&p);

        assert!(dest_matches_pinned_sha256(&p, None));
        assert!(dest_matches_pinned_sha256(&p, Some(&actual)));
        assert!(!dest_matches_pinned_sha256(&p, Some("deadbeef")));
        assert!(!dest_matches_pinned_sha256(
            &tmp.path().join("missing"),
            Some(&actual)
        ));
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

    fn make_targz(dir: &Path, entries: &[(&str, &[u8], bool)]) -> PathBuf {
        let path = dir.join("fixture.tar.gz");
        let file = std::fs::File::create(&path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        for (name, content, exec) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(content.len() as u64);
            header.set_mode(if *exec { 0o755 } else { 0o644 });
            header.set_cksum();
            tar.append_data(&mut header, name, *content).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
        path
    }

    fn make_zip(dir: &Path, entries: &[(&str, &[u8], bool)]) -> PathBuf {
        let path = dir.join("fixture.zip");
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, content, exec) in entries {
            let mut options: zip::write::SimpleFileOptions = zip::write::FileOptions::default();
            #[cfg(unix)]
            if *exec {
                options = options.unix_permissions(0o755);
            }
            zip.start_file(*name, options).unwrap();
            std::io::Write::write_all(&mut zip, content).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn pick_binary_prefers_exact_name_match() {
        let tmp = tempfile::TempDir::new().unwrap();
        let archive = make_targz(
            tmp.path(),
            &[
                ("pkg/README.md", b"docs", false),
                ("pkg/bin/rg", b"binary", true),
                ("pkg/bin/other-tool", b"other", true),
            ],
        );
        let out = tmp.path().join("extracted");
        extract_archive(&archive, "tar.gz", &out).unwrap();
        let picked = pick_binary(&out, "rg").unwrap();
        assert!(picked.ends_with("pkg/bin/rg"));
    }

    #[test]
    fn pick_binary_single_executable_wins() {
        let tmp = tempfile::TempDir::new().unwrap();
        let archive = make_targz(
            tmp.path(),
            &[
                ("pkg/README.md", b"docs", false),
                ("pkg/tool", b"binary", true),
            ],
        );
        let out = tmp.path().join("extracted");
        extract_archive(&archive, "tar.gz", &out).unwrap();
        let picked = pick_binary(&out, "anything").unwrap();
        assert!(picked.ends_with("pkg/tool"));
    }

    #[test]
    fn pick_binary_ambiguous_returns_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let archive = make_targz(
            tmp.path(),
            &[("pkg/a/tool-a", b"a", true), ("pkg/b/tool-b", b"b", true)],
        );
        let out = tmp.path().join("extracted");
        extract_archive(&archive, "tar.gz", &out).unwrap();
        let err = pick_binary(&out, "nomatch").unwrap_err();
        assert!(err.contains("multiple candidate binaries"), "{err}");
    }

    #[test]
    fn zip_archive_extracts_and_picks() {
        let tmp = tempfile::TempDir::new().unwrap();
        let archive = make_zip(
            tmp.path(),
            &[
                ("bin/protoc", b"proto-binary", true),
                ("include/readme.txt", b"docs", false),
            ],
        );
        let out = tmp.path().join("extracted");
        extract_archive(&archive, "zip", &out).unwrap();
        let picked = pick_binary(&out, "protoc").unwrap();
        assert!(picked.ends_with("bin/protoc"));
    }

    #[test]
    fn archive_pin_uses_marker_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let dest = tmp.path().join("bin").join("rg");
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, b"binary").unwrap();

        assert!(
            !installed_matches_pin("https://example.com/rg.tar.gz", Some("deadbeef"), &dest),
            "no marker file means pin is unverified"
        );

        std::fs::write(sha_marker_path(&dest), "deadbeef").unwrap();
        assert!(installed_matches_pin(
            "https://example.com/rg.tar.gz",
            Some("deadbeef"),
            &dest
        ));
        assert!(
            !installed_matches_pin("https://example.com/rg.tar.gz", Some("other"), &dest),
            "bumping the pinned sha must invalidate"
        );
    }

    #[test]
    fn archive_kind_detection() {
        assert_eq!(archive_kind("https://x/rg.tar.gz"), Some("tar.gz"));
        assert_eq!(archive_kind("https://x/rg.TGZ"), Some("tar.gz"));
        assert_eq!(archive_kind("https://x/protoc.zip"), Some("zip"));
        assert_eq!(archive_kind("https://x/rg"), None);
    }
}
