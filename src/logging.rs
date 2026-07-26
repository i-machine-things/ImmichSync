use anyhow::{Context, Result};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

pub struct Logger {
    file: Option<Mutex<std::fs::File>>,
}

impl Logger {
    pub fn to_file(path: &Path) -> Result<Logger> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
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
