use crate::config::Config;
use crate::immich::ImmichClient;
use crate::logging::Logger;
use crate::manifest::{Entry, Manifest};
use crate::scanner;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::Path;
use std::time::SystemTime;

#[derive(Default)]
pub struct SyncSummary {
    pub scanned: usize,
    pub skipped_cached: usize,
    pub already_on_server: usize,
    pub uploaded: usize,
    pub failed: usize,
}

pub fn run(
    config: &Config,
    manifest_path: &Path,
    dry_run: bool,
    log: &Logger,
) -> Result<SyncSummary> {
    let client = ImmichClient::new(&config.server_url, &config.api_key)?;
    let mut manifest = Manifest::load(manifest_path)?;
    let mut summary = SyncSummary::default();

    for dir in &config.photos_dirs {
        if !dir.exists() {
            log.log(&format!("skipping missing photos_dir: {}", dir.display()));
            continue;
        }
        log.log(&format!("scanning {}", dir.display()));
        let found = scanner::walk(dir)?;
        summary.scanned += found.len();

        // Files that need a fresh checksum + server check this run.
        let mut to_check = Vec::new();
        for f in &found {
            if manifest.matches_unchanged(&f.rel_key, f.size, f.mtime) {
                summary.skipped_cached += 1;
                continue;
            }
            match scanner::sha1_hex(&f.abs_path) {
                Ok(checksum) => to_check.push((f, checksum)),
                Err(e) => {
                    log.log(&format!("hash failed for {}: {e}", f.abs_path.display()));
                    summary.failed += 1;
                }
            }
        }

        if to_check.is_empty() {
            continue;
        }

        let check_items: Vec<(String, String)> = to_check
            .iter()
            .map(|(f, checksum)| (f.rel_key.clone(), checksum.clone()))
            .collect();
        let results = client.bulk_upload_check(&check_items)?;
        let results_by_id: std::collections::HashMap<_, _> =
            results.into_iter().map(|r| (r.id.clone(), r)).collect();

        for (f, checksum) in &to_check {
            let result = match results_by_id.get(&f.rel_key) {
                Some(r) => r,
                None => {
                    log.log(&format!("no bulk-check result for {}", f.rel_key));
                    summary.failed += 1;
                    continue;
                }
            };

            if result.action == "reject" {
                // Server already has this checksum — record it as synced without
                // re-uploading the bytes.
                summary.already_on_server += 1;
                manifest.entries.insert(
                    f.rel_key.clone(),
                    Entry {
                        size: f.size,
                        mtime: f.mtime,
                        checksum: checksum.clone(),
                        asset_id: result.asset_id.clone(),
                        status: "duplicate".to_string(),
                    },
                );
                continue;
            }

            if dry_run {
                log.log(&format!("[dry-run] would upload {}", f.rel_key));
                summary.uploaded += 1;
                continue;
            }

            let (created_at, modified_at) = file_times(&f.abs_path);
            match client.upload_asset(&f.abs_path, checksum, created_at, modified_at) {
                Ok(resp) => {
                    let status = if resp.status == "duplicate" {
                        summary.already_on_server += 1;
                        "duplicate"
                    } else {
                        summary.uploaded += 1;
                        "uploaded"
                    };
                    manifest.entries.insert(
                        f.rel_key.clone(),
                        Entry {
                            size: f.size,
                            mtime: f.mtime,
                            checksum: checksum.clone(),
                            asset_id: Some(resp.id),
                            status: status.to_string(),
                        },
                    );
                    log.log(&format!("{status}: {}", f.rel_key));
                }
                Err(e) => {
                    log.log(&format!("upload failed for {}: {e}", f.rel_key));
                    summary.failed += 1;
                }
            }
        }
    }

    if !dry_run {
        manifest.save(manifest_path)?;
    }

    Ok(summary)
}

fn file_times(path: &Path) -> (DateTime<Utc>, DateTime<Utc>) {
    let meta = std::fs::metadata(path).ok();
    let modified: DateTime<Utc> = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .into();
    let created: DateTime<Utc> = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or(modified);
    (created, modified)
}
