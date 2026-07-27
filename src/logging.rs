use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// This runs nightly for the life of the install, so the log needs a cap —
/// once it crosses this size, the previous contents move to a single `.1`
/// backup rather than growing forever. Checked both at startup and on every
/// write, since a single large first-time backfill of an existing photo
/// library can log tens of thousands of lines in one run.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub struct Logger {
    file: Option<Mutex<std::fs::File>>,
    path: Option<PathBuf>,
}

impl Logger {
    pub fn to_file(path: &Path) -> Result<Logger> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        rotate_if_too_large(path)?;
        let file = open_append(path)?;
        Ok(Logger {
            file: Some(Mutex::new(file)),
            path: Some(path.to_path_buf()),
        })
    }

    pub fn log(&self, line: &str) {
        let stamped = format!("[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), line);
        println!("{stamped}");
        let (Some(f), Some(path)) = (&self.file, &self.path) else {
            return;
        };
        let Ok(mut f) = f.lock() else {
            return;
        };
        if let Ok(meta) = f.metadata()
            && meta.len() >= MAX_LOG_BYTES
            && let Ok(rotated) = rotate_and_reopen(path)
        {
            *f = rotated;
        }
        let _ = writeln!(f, "{stamped}");
    }
}

fn open_append(path: &Path) -> Result<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))
}

fn rename_to_backup(path: &Path) -> Result<()> {
    let rotated = PathBuf::from(format!("{}.1", path.display()));
    std::fs::rename(path, &rotated)
        .with_context(|| format!("rotating {} to {}", path.display(), rotated.display()))
}

fn rotate_and_reopen(path: &Path) -> Result<std::fs::File> {
    rename_to_backup(path)?;
    open_append(path)
}

fn rotate_if_too_large(path: &Path) -> Result<()> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).with_context(|| format!("reading metadata for {}", path.display()));
        }
    };
    if meta.len() <= MAX_LOG_BYTES {
        return Ok(());
    }
    rename_to_backup(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_mid_run_when_size_exceeds_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let logger = Logger::to_file(&path).unwrap();

        // Each line is padded so the total crosses MAX_LOG_BYTES well within a
        // reasonable number of log() calls, simulating a single large run.
        let line = "x".repeat(1024);
        let lines_needed = (MAX_LOG_BYTES / 1024) as usize + 10;
        for _ in 0..lines_needed {
            logger.log(&line);
        }

        let backup = PathBuf::from(format!("{}.1", path.display()));
        assert!(
            backup.exists(),
            "expected a .1 backup after crossing the cap mid-run"
        );
        let active_len = std::fs::metadata(&path).unwrap().len();
        assert!(
            active_len < MAX_LOG_BYTES,
            "active log should be small again after rotation, got {active_len} bytes"
        );
    }
}
