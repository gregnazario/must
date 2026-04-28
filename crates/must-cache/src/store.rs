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
        format!("{}:{}:{}", key.recipe, key.target, key.profile).into_bytes()
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
        let entry_dir = self.entry_dir(key);
        std::fs::create_dir_all(&entry_dir).map_err(Error::Io)?;

        for output in outputs {
            if output.exists() {
                let file_name = output.file_name().unwrap_or_default();
                let dest = entry_dir.join(file_name);
                std::fs::copy(output, &dest).map_err(Error::Io)?;
            }
        }

        let sled_key = Self::sled_key(key);
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
}
