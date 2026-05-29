use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::error::Result;
use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;
use tracing::{debug, warn};

use super::{DownloadProgress, DownloadTask};
use crate::config::DownloadMirror;

use super::mirror::{build_fallback_chain, transform_url};

pub type ProgressCallback = Arc<dyn Fn(&DownloadProgress) + Send + Sync>;

const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 30;

pub struct DownloadManager {
    http: reqwest::Client,
    fallback_chain: Vec<DownloadMirror>,
    max_concurrent: usize,
    progress: Arc<Mutex<DownloadProgress>>,
    on_progress: Option<ProgressCallback>,
}

impl DownloadManager {
    pub fn new(mirror: DownloadMirror, max_concurrent: usize) -> Self {
        let fallback_chain = build_fallback_chain(&mirror);
        Self {
            http: reqwest::Client::builder()
                .user_agent(concat!("MiaoMinecraftLauncher/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
                .connect_timeout(Duration::from_secs(10))
                .build()
                .expect("failed to build HTTP client"),
            fallback_chain,
            max_concurrent,
            progress: Arc::new(Mutex::new(DownloadProgress {
                total_bytes: 0,
                downloaded_bytes: 0,
                total_files: 0,
                completed_files: 0,
                current_file: String::new(),
            })),
            on_progress: None,
        }
    }

    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.on_progress = Some(callback);
        self
    }

    pub async fn download_all(&self, tasks: Vec<DownloadTask>) -> Result<()> {
        {
            let mut progress = self.progress.lock().await;
            progress.total_files = tasks.len();
            progress.total_bytes = tasks.iter().filter_map(|t| t.size).sum();
        }

        stream::iter(tasks)
            .map(|task| self.download_single(task))
            .buffer_unordered(self.max_concurrent)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    async fn download_single(&self, task: DownloadTask) -> Result<()> {
        if let Some(parent) = task.dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if task.dest.exists() && file_matches_checksums(&task).await? {
            let mut progress = self.progress.lock().await;
            progress.completed_files += 1;
            return Ok(());
        }

        {
            let mut progress = self.progress.lock().await;
            progress.current_file = task
                .dest
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
        }

        let bytes = self.download_with_fallback(&task.url).await?;

        tokio::fs::write(&task.dest, &bytes).await?;

        if let Some(expected_sha1) = &task.sha1
            && !verify_sha1(&task.dest, expected_sha1).await?
        {
            return Err(crate::error::MiaoError::Other(format!(
                "SHA1 mismatch for {}: expected {}",
                task.dest.display(),
                expected_sha1
            )));
        }
        if let Some(expected_sha256) = &task.sha256
            && !verify_sha256(&task.dest, expected_sha256).await?
        {
            return Err(crate::error::MiaoError::Other(format!(
                "SHA256 mismatch for {}: expected {}",
                task.dest.display(),
                expected_sha256
            )));
        }

        let mut progress = self.progress.lock().await;
        progress.completed_files += 1;
        progress.downloaded_bytes += task.size.unwrap_or(bytes.len() as u64);

        if let Some(ref cb) = self.on_progress {
            cb(&progress);
        }

        Ok(())
    }

    async fn download_with_fallback(&self, original_url: &str) -> Result<bytes::Bytes> {
        let mut last_error = None;

        for mirror in &self.fallback_chain {
            let url = transform_url(original_url, mirror);

            match self.download_with_retry(&url).await {
                Ok(bytes) => return Ok(bytes),
                Err(e) => {
                    warn!(
                        url = %url,
                        mirror = ?mirror,
                        error = %e,
                        "Download failed, trying next mirror"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            crate::error::MiaoError::Other(format!("All mirrors exhausted for {}", original_url))
        }))
    }

    async fn download_with_retry(&self, url: &str) -> Result<bytes::Bytes> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = Duration::from_millis(BASE_RETRY_DELAY_MS * 2u64.pow(attempt - 1));
                debug!(
                    attempt,
                    delay_ms = delay.as_millis(),
                    url,
                    "Retrying download"
                );
                tokio::time::sleep(delay).await;
            }

            match self.http.get(url).send().await {
                Ok(resp) => match resp.error_for_status() {
                    Ok(resp) => match resp.bytes().await {
                        Ok(bytes) => return Ok(bytes),
                        Err(e) => last_error = Some(crate::error::MiaoError::Http(e)),
                    },
                    Err(e) => {
                        if is_non_retryable(&e) {
                            return Err(crate::error::MiaoError::Http(e));
                        }
                        last_error = Some(crate::error::MiaoError::Http(e));
                    }
                },
                Err(e) => last_error = Some(crate::error::MiaoError::Http(e)),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            crate::error::MiaoError::Other(format!(
                "Download failed after {} retries: {}",
                MAX_RETRIES, url
            ))
        }))
    }

    pub async fn progress(&self) -> DownloadProgress {
        self.progress.lock().await.clone()
    }
}

fn is_non_retryable(err: &reqwest::Error) -> bool {
    if let Some(status) = err.status() {
        matches!(status.as_u16(), 400 | 401 | 403 | 404 | 410)
    } else {
        false
    }
}

pub async fn verify_sha1(path: &Path, expected: &str) -> Result<bool> {
    use sha1::{Digest, Sha1};

    let data = tokio::fs::read(path).await?;
    let hash = Sha1::digest(&data);
    let hex = format!("{:x}", hash);
    Ok(hex == expected)
}

pub async fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    use sha2::{Digest, Sha256};

    let data = tokio::fs::read(path).await?;
    let hash = Sha256::digest(&data);
    let hex = format!("{:x}", hash);
    Ok(hex.eq_ignore_ascii_case(expected))
}

async fn file_matches_checksums(task: &DownloadTask) -> Result<bool> {
    if let Some(expected) = &task.sha1
        && !verify_sha1(&task.dest, expected).await?
    {
        return Ok(false);
    }
    if let Some(expected) = &task.sha256
        && !verify_sha256(&task.dest, expected).await?
    {
        return Ok(false);
    }
    Ok(task.sha1.is_some() || task.sha256.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn verify_sha1_correct_hash() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();

        let expected = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
        let result = verify_sha1(file.path(), expected).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn verify_sha1_incorrect_hash() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"hello world").unwrap();

        let result = verify_sha1(file.path(), "0000000000000000000000000000000000000000")
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn download_manager_progress_initial_state() {
        let manager = DownloadManager::new(DownloadMirror::Official, 4);
        let progress = manager.progress().await;
        assert_eq!(progress.total_files, 0);
        assert_eq!(progress.completed_files, 0);
        assert_eq!(progress.total_bytes, 0);
        assert_eq!(progress.downloaded_bytes, 0);
    }

    #[tokio::test]
    async fn download_manager_skips_verified_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"hello world").unwrap();

        let manager = DownloadManager::new(DownloadMirror::Official, 4);

        let tasks = vec![DownloadTask {
            url: "https://httpbin.org/status/404".to_string(),
            dest: file_path,
            sha1: Some("2aae6c35c94fcfb415dbe95f408b9ce91ee846ed".to_string()),
            sha256: None,
            size: Some(11),
        }];

        let result = manager.download_all(tasks).await;
        assert!(result.is_ok());

        let progress = manager.progress().await;
        assert_eq!(progress.completed_files, 1);
    }
}
