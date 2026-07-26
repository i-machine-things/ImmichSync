use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REPO: &str = "i-machine-things/ImmichSync";
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 3600);

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    html_url: String,
}

pub struct UpdateInfo {
    pub tag: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Default)]
struct CheckCache {
    last_checked_unix: u64,
}

/// Compares dotted version strings numerically (ignoring a leading 'v' and any
/// '-suffix' pre-release tag), e.g. "v1.2.3-rc1" -> (1, 2, 3).
fn version_tuple(v: &str) -> (u64, u64, u64) {
    let numeric = v.trim_start_matches('v').split('-').next().unwrap_or("0");
    let mut parts = numeric.split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
        parts.next().unwrap_or(0),
    )
}

fn fetch_latest() -> Result<GhRelease> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("immichsync")
        .build()?;
    let resp = client
        .get(format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .header("Accept", "application/vnd.github+json")
        .send()
        .context("contacting GitHub releases API")?;
    if !resp.status().is_success() {
        anyhow::bail!("GitHub releases API returned HTTP {}", resp.status());
    }
    resp.json().context("parsing releases API response")
}

/// Manual, unconditional check (used by `immichsync update check`).
pub fn check_now() -> Result<Option<UpdateInfo>> {
    let release = fetch_latest()?;
    let current = version_tuple(env!("CARGO_PKG_VERSION"));
    if version_tuple(&release.tag_name) > current {
        Ok(Some(UpdateInfo {
            tag: release.tag_name,
            url: release.html_url,
        }))
    } else {
        Ok(None)
    }
}

/// Best-effort check used from `run`: silently does nothing on any error, and
/// skips the network call entirely if we already checked within the last 24h.
pub fn check_if_due(cache_path: &Path) -> Option<UpdateInfo> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

    if let Ok(text) = std::fs::read_to_string(cache_path)
        && let Ok(cache) = serde_json::from_str::<CheckCache>(&text)
        && now.saturating_sub(cache.last_checked_unix) < CHECK_INTERVAL.as_secs()
    {
        return None;
    }

    // Only stamp the cache on a successful check — otherwise a transient
    // network failure would suppress retries for the rest of the interval
    // instead of just skipping this one run.
    let outcome = check_now();
    if outcome.is_ok() {
        let cache = CheckCache {
            last_checked_unix: now,
        };
        if let Ok(text) = serde_json::to_string(&cache) {
            let _ = std::fs::write(cache_path, text);
        }
    }
    outcome.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::version_tuple;

    #[test]
    fn parses_plain_versions() {
        assert_eq!(version_tuple("v1.2.3"), (1, 2, 3));
        assert_eq!(version_tuple("1.2.3"), (1, 2, 3));
    }

    #[test]
    fn strips_prerelease_suffix() {
        assert_eq!(version_tuple("v2.0.0-rc1"), (2, 0, 0));
    }

    #[test]
    fn orders_correctly() {
        assert!(version_tuple("v1.10.0") > version_tuple("v1.9.0"));
        assert!(version_tuple("v2.0.0") > version_tuple("v1.99.99"));
        assert_eq!(version_tuple("v1.0.0"), version_tuple("v1.0.0"));
    }

    #[test]
    fn missing_parts_default_to_zero() {
        assert_eq!(version_tuple("v1"), (1, 0, 0));
        assert_eq!(version_tuple(""), (0, 0, 0));
    }
}
