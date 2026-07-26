use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// This runs nightly for the life of the install, so the log needs a cap —
/// once it crosses this size, the previous contents move to a single `.1`
/// backup rather than growing forever.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub struct Logger {
    file: Option<Mutex<std::fs::File>>,
}

impl Logger {
    pub fn to_file(path: &Path) -> Result<Logger> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        rotate_if_too_large(path)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("opening {}", path.display()))?;
        Ok(Logger {
            file: Some(Mutex::new(file)),
        })
    }

    pub fn log(&self, line: &str) {
        let stamped = format!("[{}] {}", Local::now().format("%Y-%m-%d %H:%M:%S"), line);
        println!("{stamped}");
        if let Some(f) = &self.file
            && let Ok(mut f) = f.lock()
        {
            let _ = writeln!(f, "{stamped}");
        }
    }
}

fn rotate_if_too_large(path: &Path) -> Result<()> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(());
    };
    if meta.len() <= MAX_LOG_BYTES {
        return Ok(());
    }
    let rotated = PathBuf::from(format!("{}.1", path.display()));
    std::fs::rename(path, &rotated)
        .with_context(|| format!("rotating {} to {}", path.display(), rotated.display()))
}
