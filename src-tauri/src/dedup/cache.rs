use std::collections::HashMap;
use std::path::Path;

use crate::error::Result;

/// SHA-256 hex of a whole file -> its acoustic fingerprint.
///
/// Keying by content hash rather than path means the cache self-invalidates:
/// an edited or re-encoded file simply hashes to a new key, and a renamed
/// but otherwise untouched file still hits.
pub type FingerprintCache = HashMap<String, Vec<u32>>;

/// Loads the cache from disk, or starts empty if it is missing or corrupt —
/// this is a speed-up, not a source of truth, so a bad file is never fatal.
pub fn load(path: &Path) -> FingerprintCache {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

/// Writes the cache back to disk, swapping in a temp file so a crash
/// mid-write cannot leave a corrupt cache behind.
pub fn save(path: &Path, cache: &FingerprintCache) -> Result<()> {
    let json = serde_json::to_string(cache)?;
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "notata-fingerprint-cache-{}-{}.json",
            tag,
            std::process::id()
        ))
    }

    #[test]
    fn missing_file_loads_as_empty() {
        let path = temp_path("missing");
        std::fs::remove_file(&path).ok();

        assert!(load(&path).is_empty());
    }

    #[test]
    fn corrupt_file_loads_as_empty_instead_of_failing() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"not json").unwrap();

        assert!(load(&path).is_empty());

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn round_trips_through_save_and_load() {
        let path = temp_path("roundtrip");
        let mut cache = FingerprintCache::new();
        cache.insert("deadbeef".to_string(), vec![1, 2, 3]);

        save(&path, &cache).unwrap();
        let loaded = load(&path);

        assert_eq!(loaded.get("deadbeef"), Some(&vec![1, 2, 3]));
        assert!(!path.with_extension("json.tmp").exists());

        std::fs::remove_file(&path).ok();
    }
}
