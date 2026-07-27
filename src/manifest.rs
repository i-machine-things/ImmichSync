use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// What we know about one local file's last-synced state, keyed by its path
/// (as a string) relative to the photos directory it was found under.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entry {
    pub size: u64,
    pub mtime: i64,
    pub checksum: String,
    pub asset_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(flatten)]
    pub entries: HashMap<String, Entry>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Manifest> {
        if !path.exists() {
            return Ok(Manifest::default());
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        if text.trim().is_empty() {
            return Ok(Manifest::default());
        }
        let manifest: Manifest =
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(manifest)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
    }

    /// True if `entry` for `key` already matches what's recorded (same size and
    /// mtime) and was previously confirmed present on the server — safe to skip
    /// re-hashing and re-checking this file.
    pub fn matches_unchanged(&self, key: &str, size: u64, mtime: i64) -> bool {
        match self.entries.get(key) {
            Some(e) => {
                e.size == size
                    && e.mtime == mtime
                    && (e.status == "uploaded" || e.status == "duplicate")
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synced_entry(size: u64, mtime: i64, status: &str) -> Entry {
        Entry {
            size,
            mtime,
            checksum: "deadbeef".to_string(),
            asset_id: Some("asset-1".to_string()),
            status: status.to_string(),
        }
    }

    #[test]
    fn unknown_key_is_not_unchanged() {
        let m = Manifest::default();
        assert!(!m.matches_unchanged("foo.jpg", 100, 1000));
    }

    #[test]
    fn matching_uploaded_entry_is_unchanged() {
        let mut m = Manifest::default();
        m.entries
            .insert("foo.jpg".to_string(), synced_entry(100, 1000, "uploaded"));
        assert!(m.matches_unchanged("foo.jpg", 100, 1000));
    }

    #[test]
    fn matching_duplicate_entry_is_unchanged() {
        let mut m = Manifest::default();
        m.entries
            .insert("foo.jpg".to_string(), synced_entry(100, 1000, "duplicate"));
        assert!(m.matches_unchanged("foo.jpg", 100, 1000));
    }

    #[test]
    fn changed_size_or_mtime_is_not_unchanged() {
        let mut m = Manifest::default();
        m.entries
            .insert("foo.jpg".to_string(), synced_entry(100, 1000, "uploaded"));
        assert!(!m.matches_unchanged("foo.jpg", 101, 1000));
        assert!(!m.matches_unchanged("foo.jpg", 100, 1001));
    }

    #[test]
    fn round_trips_through_disk() {
        let mut m = Manifest::default();
        m.entries
            .insert("foo.jpg".to_string(), synced_entry(100, 1000, "uploaded"));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("manifest.json");
        m.save(&path).unwrap();
        let loaded = Manifest::load(&path).unwrap();
        assert!(loaded.matches_unchanged("foo.jpg", 100, 1000));
    }
}
