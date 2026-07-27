use anyhow::{Context, Result};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

pub struct FoundFile {
    pub abs_path: PathBuf,
    /// Path relative to the photos_dir root, using `/` as separator so the
    /// manifest key is stable across Linux and Windows.
    pub rel_key: String,
    pub size: u64,
    pub mtime: i64,
}

/// Recursively lists regular files under `root`, skipping hidden files/dirs
/// (dotfiles) and zero-byte files.
pub fn walk(root: &Path) -> Result<Vec<FoundFile>> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e))
    {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                if e.depth() == 0 {
                    // The root photos_dir itself is unreadable — propagate as a fatal error.
                    return Err(e).with_context(|| format!("walking {}", root.display()));
                }
                // A subdirectory or file inside the tree is inaccessible (e.g.,
                // permission denied on a system folder). Skip it and continue.
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) if entry.depth() > 0 => continue,
            Err(e) => return Err(e).with_context(|| format!("walking {}", root.display())),
        };
        if meta.len() == 0 {
            continue;
        }
        let abs_path = entry.path().to_path_buf();
        let rel = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_string_lossy()
            .replace('\\', "/");
        let mtime = meta
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(FoundFile {
            abs_path,
            rel_key: rel,
            size: meta.len(),
            mtime,
        });
    }
    Ok(out)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
        if let Ok(meta) = entry.metadata()
            && meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
        {
            return true;
        }
    }
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Hex-encoded SHA1 of the file contents, matching the checksum format Immich
/// uses for its bulk-upload-check / x-immich-checksum dedup logic.
pub fn sha1_hex(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha1_matches_known_vector() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abc.txt");
        std::fs::File::create(&path)
            .unwrap()
            .write_all(b"abc")
            .unwrap();
        assert_eq!(
            sha1_hex(&path).unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn walk_skips_hidden_files_and_empty_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("visible.jpg"), b"data").unwrap();
        std::fs::write(dir.path().join(".hidden.jpg"), b"data").unwrap();
        std::fs::write(dir.path().join("empty.jpg"), b"").unwrap();

        let found = walk(dir.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rel_key, "visible.jpg");
    }
}
