use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("", "", "immichsync").context("could not determine home directory")
}

pub fn config_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

pub fn data_dir() -> Result<PathBuf> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn manifest_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("manifest.json"))
}

pub fn log_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("immichsync.log"))
}

pub fn update_cache_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("update_check.json"))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub server_url: String,
    pub api_key: String,
    pub photos_dirs: Vec<PathBuf>,
}

impl Config {
    pub fn exists() -> Result<bool> {
        Ok(config_path()?.exists())
    }

    pub fn load() -> Result<Config> {
        let path = config_path()?;
        if !path.exists() {
            bail!(
                "no config found at {} — run `immichsync init` first",
                path.display()
            );
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let config: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self)?;
        write_config_file(&path, &text)
    }
}

/// Writes the config (which holds a plaintext API key) with restrictive
/// permissions from the moment the file is created, rather than writing with
/// default permissions and narrowing them afterwards — the latter leaves a
/// window where the secret is readable at the OS-default file mode.
#[cfg(unix)]
fn write_config_file(path: &std::path::Path, text: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("writing {}", path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("writing {}", path.display()))
}

#[cfg(not(unix))]
fn write_config_file(path: &std::path::Path, text: &str) -> Result<()> {
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}
