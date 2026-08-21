use must_core::{Cache, CacheKey, CacheLookup, Error, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct DiskCache {
    cache_dir: PathBuf,
    db: sled::Db,
}

impl DiskCache {
    pub fn open(cache_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(cache_dir).map_err(Error::Io)?;
        let db_path = cache_dir.join("index.sled");
        let db = sled::open(&db_path).map_err(|e| Error::Cache(e.to_string()))?;
        Ok(Self {
            cache_dir: cache_dir.to_owned(),
            db,
        })
    }

    fn entry_dir(&self, key: &CacheKey) -> PathBuf {
        let hash = &key.hash;
        let prefix = &hash[..2.min(hash.len())];
        let rest = &hash[2.min(hash.len())..];
        self.cache_dir.join(prefix).join(rest)
    }

    fn sled_key(key: &CacheKey) -> Vec<u8> {
        let mut out = Vec::new();
        for part in [&key.recipe, &key.target, &key.profile] {
            out.extend_from_slice(&(part.len() as u64).to_be_bytes());
            out.extend_from_slice(part.as_bytes());
        }
        out
    }

    fn parse_sled_key(bytes: &[u8]) -> Option<(String, String, String)> {
        let mut parts = Vec::with_capacity(3);
        let mut rest = bytes;
        for _ in 0..3 {
            if rest.len() < 8 {
                return None;
            }
            let (len_bytes, tail) = rest.split_at(8);
            let len = u64::from_be_bytes(len_bytes.try_into().ok()?) as usize;
            rest = tail;
            if rest.len() < len {
                return None;
            }
            let (part, tail) = rest.split_at(len);
            parts.push(String::from_utf8(part.to_vec()).ok()?);
            rest = tail;
        }
        if !rest.is_empty() {
            return None;
        }
        let (recipe, target, profile) = (parts[0].clone(), parts[1].clone(), parts[2].clone());
        Some((recipe, target, profile))
    }
}

impl Cache for DiskCache {
    fn lookup(&self, key: &CacheKey) -> Result<CacheLookup> {
        let sled_key = Self::sled_key(key);
        match self
            .db
            .get(&sled_key)
            .map_err(|e| Error::Cache(e.to_string()))?
        {
            None => Ok(CacheLookup::Miss),
            Some(stored_hash) => {
                if stored_hash.as_ref() == key.hash.as_bytes() {
                    let entry_dir = self.entry_dir(key);
                    if entry_dir.exists() {
                        Ok(CacheLookup::Hit)
                    } else {
                        Ok(CacheLookup::Stale)
                    }
                } else {
                    Ok(CacheLookup::Stale)
                }
            }
        }
    }

    fn store(&self, key: &CacheKey, outputs: &[PathBuf]) -> Result<()> {
        let sled_key = Self::sled_key(key);
        if let Ok(Some(old_hash)) = self.db.get(&sled_key)
            && old_hash.as_ref() != key.hash.as_bytes()
        {
            let old_key = CacheKey {
                recipe: key.recipe.clone(),
                target: key.target.clone(),
                profile: key.profile.clone(),
                hash: String::from_utf8_lossy(&old_hash).into_owned(),
            };
            let old_dir = self.entry_dir(&old_key);
            if old_dir.exists() {
                let _ = std::fs::remove_dir_all(&old_dir);
            }
        }

        let entry_dir = self.entry_dir(key);
        std::fs::create_dir_all(&entry_dir).map_err(Error::Io)?;

        let mut used_names = std::collections::HashSet::new();
        for output in outputs {
            if output.exists() {
                let file_name = output
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let dest_name = if used_names.insert(file_name.clone()) {
                    file_name
                } else {
                    let digest = hash_string(&output.to_string_lossy());
                    format!("{}-{}", &digest[..16], file_name)
                };
                let dest = entry_dir.join(dest_name);
                std::fs::copy(output, &dest).map_err(Error::Io)?;
            }
        }

        self.db
            .insert(sled_key, key.hash.as_bytes())
            .map_err(|e| Error::Cache(e.to_string()))?;
        Ok(())
    }

    fn invalidate(&self, key: &CacheKey) -> Result<()> {
        let sled_key = Self::sled_key(key);
        self.db
            .remove(sled_key)
            .map_err(|e| Error::Cache(e.to_string()))?;
        let entry_dir = self.entry_dir(key);
        if entry_dir.exists() {
            std::fs::remove_dir_all(&entry_dir).map_err(Error::Io)?;
        }
        Ok(())
    }
}

impl DiskCache {
    pub fn list_entries(&self) -> Result<Vec<(String, String, String, String)>> {
        let mut entries = Vec::new();
        for item in self.db.iter() {
            let (key_bytes, value_bytes) = item.map_err(|e| Error::Cache(e.to_string()))?;
            if let Some((recipe, target, profile)) = Self::parse_sled_key(&key_bytes) {
                let hash = String::from_utf8_lossy(&value_bytes).to_string();
                entries.push((recipe, target, profile, hash));
            }
        }
        Ok(entries)
    }

    pub fn invalidate_all(&self) -> Result<usize> {
        let entries = self.list_entries()?;
        let count = entries.len();
        for (recipe, target, profile, hash) in &entries {
            let key = CacheKey {
                recipe: recipe.clone(),
                target: target.clone(),
                profile: profile.clone(),
                hash: hash.clone(),
            };
            self.invalidate(&key)?;
        }
        Ok(count)
    }
}

pub fn hash_string(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use must_core::{Cache, CacheLookup};
    use tempfile::TempDir;

    fn make_key(hash: &str) -> CacheKey {
        CacheKey {
            recipe: "my-recipe".to_string(),
            target: "host".to_string(),
            profile: "debug".to_string(),
            hash: hash.to_string(),
        }
    }

    #[test]
    fn test_lookup_miss_on_empty() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Miss));
    }

    #[test]
    fn test_store_and_hit() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        cache.store(&key, &[]).unwrap();
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Hit));
    }

    #[test]
    fn test_store_copies_output_file() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        let output_file = dir.path().join("output.txt");
        std::fs::write(&output_file, b"hello").unwrap();
        cache.store(&key, &[output_file]).unwrap();
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Hit));
        // Verify the file was copied into the entry dir
        let entry = cache.entry_dir(&key);
        assert!(entry.join("output.txt").exists());
    }

    #[test]
    fn test_lookup_stale_when_hash_changes() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key1 = make_key("hash-one");
        cache.store(&key1, &[]).unwrap();
        // Same recipe/target/profile but different hash
        let key2 = make_key("hash-two");
        assert!(matches!(cache.lookup(&key2).unwrap(), CacheLookup::Stale));
    }

    #[test]
    fn test_invalidate_returns_miss() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        cache.store(&key, &[]).unwrap();
        cache.invalidate(&key).unwrap();
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Miss));
    }

    #[test]
    fn test_invalidate_nonexistent_is_ok() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        assert!(cache.invalidate(&key).is_ok());
    }

    #[test]
    fn test_stale_when_entry_dir_deleted() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        cache.store(&key, &[]).unwrap();
        // Manually remove the entry directory to simulate stale state
        let entry_dir = cache.entry_dir(&key);
        std::fs::remove_dir_all(&entry_dir).unwrap();
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Stale));
    }

    #[test]
    fn test_store_with_nonexistent_output_skips_copy() {
        // When an output path doesn't exist, store() should skip the copy but still record the key
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("no-output");
        let nonexistent = dir.path().join("does-not-exist.bin");
        cache.store(&key, &[nonexistent]).unwrap();
        assert!(matches!(cache.lookup(&key).unwrap(), CacheLookup::Hit));
    }

    #[test]
    fn test_hash_string_is_deterministic_hex() {
        let h1 = hash_string("hello");
        let h2 = hash_string("hello");
        assert_eq!(h1, h2, "hash_string must be deterministic");
        // SHA-256 produces a 64-character hex string
        assert_eq!(h1.len(), 64);
        assert!(h1.chars().all(|c| c.is_ascii_hexdigit()));

        let h3 = hash_string("world");
        assert_ne!(h1, h3, "different inputs must produce different hashes");
    }

    #[test]
    fn test_store_removes_old_entry_dir_on_hash_change() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key1 = make_key("hash-one");
        let out1 = dir.path().join("one.txt");
        std::fs::write(&out1, b"one").unwrap();
        cache.store(&key1, &[out1]).unwrap();
        let old_entry = cache.entry_dir(&key1);
        assert!(old_entry.exists());

        let key2 = CacheKey {
            hash: "hash-two".to_string(),
            ..make_key("hash-one")
        };
        cache.store(&key2, &[]).unwrap();

        assert!(!old_entry.exists(), "orphaned entry dir must be reclaimed");
    }

    #[test]
    fn test_store_disambiguates_same_named_outputs() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let key = make_key("abc123");
        let a = dir.path().join("out/a.bin");
        let b = dir.path().join("dist/a.bin");
        std::fs::create_dir_all(a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(b.parent().unwrap()).unwrap();
        std::fs::write(&a, b"AAA").unwrap();
        std::fs::write(&b, b"BBB").unwrap();

        cache.store(&key, &[a, b]).unwrap();

        let entry = cache.entry_dir(&key);
        let copies: Vec<String> = std::fs::read_dir(&entry)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            copies.len(),
            2,
            "same-named outputs must not overwrite each other: {copies:?}"
        );
    }

    #[test]
    fn test_sled_key_roundtrips_names_with_separator() {
        let key = CacheKey {
            recipe: "a:b".to_string(),
            target: "c".to_string(),
            profile: "default".to_string(),
            hash: "x".to_string(),
        };
        let encoded = DiskCache::sled_key(&key);
        let (recipe, target, profile) = DiskCache::parse_sled_key(&encoded).unwrap();
        assert_eq!(recipe, "a:b");
        assert_eq!(target, "c");
        assert_eq!(profile, "default");
    }

    #[test]
    fn test_list_entries_and_invalidate_all_clear_everything() {
        let dir = TempDir::new().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let k1 = CacheKey {
            recipe: "a:b".to_string(),
            target: "c".to_string(),
            profile: "default".to_string(),
            hash: "hash-1".to_string(),
        };
        let k2 = make_key("hash-2");
        cache.store(&k1, &[]).unwrap();
        cache.store(&k2, &[]).unwrap();

        let entries = cache.list_entries().unwrap();
        assert_eq!(entries.len(), 2, "all keys must be listed: {entries:?}");

        let removed = cache.invalidate_all().unwrap();
        assert_eq!(removed, 2);
        assert!(cache.list_entries().unwrap().is_empty());
    }
}
