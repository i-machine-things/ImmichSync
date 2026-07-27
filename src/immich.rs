use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use reqwest::blocking::{Client, multipart};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Large videos over typical home upload bandwidth can take much longer than
/// the client's default short timeout (used for quick metadata calls), so
/// uploads get their own longer per-request timeout.
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub struct ImmichClient {
    base_url: String,
    api_key: String,
    http: Client,
}

#[derive(Serialize)]
struct BulkCheckItem<'a> {
    id: &'a str,
    checksum: String,
}

#[derive(Serialize)]
struct BulkCheckRequest<'a> {
    assets: Vec<BulkCheckItem<'a>>,
}

#[derive(Deserialize, Debug)]
pub struct BulkCheckResult {
    pub id: String,
    pub action: String,
    #[serde(rename = "assetId")]
    pub asset_id: Option<String>,
}

#[derive(Deserialize)]
struct BulkCheckResponse {
    results: Vec<BulkCheckResult>,
}

#[derive(Deserialize)]
pub struct UploadResponse {
    pub id: String,
    pub status: String,
}

impl ImmichClient {
    pub fn new(server_url: &str, api_key: &str) -> Result<ImmichClient> {
        let http = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("building HTTP client")?;
        Ok(ImmichClient {
            base_url: format!("{}/api", server_url.trim_end_matches('/')),
            api_key: api_key.to_string(),
            http,
        })
    }

    /// Unauthenticated connectivity check.
    pub fn ping(&self) -> Result<()> {
        let resp = self
            .http
            .get(format!("{}/server/ping", self.base_url))
            .send()
            .context("contacting Immich server")?;
        if !resp.status().is_success() {
            bail!("server ping failed: HTTP {}", resp.status());
        }
        Ok(())
    }

    /// Confirms the API key is valid by fetching the current user.
    pub fn validate_api_key(&self) -> Result<String> {
        let resp = self
            .http
            .get(format!("{}/users/me", self.base_url))
            .header("x-api-key", &self.api_key)
            .send()
            .context("validating API key")?;
        if !resp.status().is_success() {
            bail!("API key rejected: HTTP {}", resp.status());
        }
        let body: serde_json::Value = resp.json().context("parsing /users/me response")?;
        Ok(body
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string())
    }

    /// Given (client_id, sha1_hex) pairs, returns which are already on the
    /// server ("duplicate") vs need uploading ("accept"), batched to keep
    /// request payloads reasonable.
    pub fn bulk_upload_check(&self, items: &[(String, String)]) -> Result<Vec<BulkCheckResult>> {
        let mut all_results = Vec::with_capacity(items.len());
        for chunk in items.chunks(200) {
            let req = BulkCheckRequest {
                assets: chunk
                    .iter()
                    .map(|(id, checksum)| BulkCheckItem {
                        id,
                        checksum: checksum.clone(),
                    })
                    .collect(),
            };
            let resp = self
                .http
                .post(format!("{}/assets/bulk-upload-check", self.base_url))
                .header("x-api-key", &self.api_key)
                .json(&req)
                .send()
                .context("calling bulk-upload-check")?;
            if !resp.status().is_success() {
                bail!("bulk-upload-check failed: HTTP {}", resp.status());
            }
            let parsed: BulkCheckResponse =
                resp.json().context("parsing bulk-upload-check response")?;
            all_results.extend(parsed.results);
        }
        Ok(all_results)
    }

    pub fn upload_asset(
        &self,
        path: &Path,
        checksum_hex: &str,
        created_at: DateTime<Utc>,
        modified_at: DateTime<Utc>,
    ) -> Result<UploadResponse> {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "asset".to_string());
        let form = multipart::Form::new()
            .text(
                "fileCreatedAt",
                created_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            )
            .text(
                "fileModifiedAt",
                modified_at.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            )
            .text("filename", filename)
            .file("assetData", path)
            .with_context(|| format!("reading {}", path.display()))?;

        let resp = self
            .http
            .post(format!("{}/assets", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("x-immich-checksum", checksum_hex)
            .timeout(UPLOAD_TIMEOUT)
            .multipart(form)
            .send()
            .with_context(|| format!("uploading {}", path.display()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            bail!("upload failed: HTTP {status} — {body}");
        }
        resp.json().context("parsing upload response")
    }
}
