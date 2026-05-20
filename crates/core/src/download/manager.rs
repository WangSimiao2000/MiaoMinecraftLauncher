use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use futures::stream::{self, StreamExt};
use tokio::sync::Mutex;

use super::{DownloadProgress, DownloadTask};
use crate::config::DownloadMirror;

use super::mirror::transform_url;

pub struct DownloadManager {
    http: reqwest::Client,
    mirror: DownloadMirror,
    max_concurrent: usize,
    progress: Arc<Mutex<DownloadProgress>>,
}

impl DownloadManager {
    pub fn new(mirror: DownloadMirror, max_concurrent: usize) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("MiaoMinecraftLauncher/0.1.0")
                .build()
                .expect("failed to build HTTP client"),
            mirror,
            max_concurrent,
            progress: Arc::new(Mutex::new(DownloadProgress {
                total_bytes: 0,
                downloaded_bytes: 0,
                total_files: 0,
                completed_files: 0,
                current_file: String::new(),
            })),
        }
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
        let url = transform_url(&task.url, &self.mirror);

        if let Some(parent) = task.dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        if task.dest.exists() {
            if let Some(expected_sha1) = &task.sha1 {
                if verify_sha1(&task.dest, expected_sha1).await? {
                    let mut progress = self.progress.lock().await;
                    progress.completed_files += 1;
                    return Ok(());
                }
            }
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

        let response = self.http.get(&url).send().await?.error_for_status()?;
        let bytes = response.bytes().await?;

        tokio::fs::write(&task.dest, &bytes).await?;

        if let Some(expected_sha1) = &task.sha1 {
            if !verify_sha1(&task.dest, expected_sha1).await? {
                anyhow::bail!(
                    "SHA1 mismatch for {}: expected {}",
                    task.dest.display(),
                    expected_sha1
                );
            }
        }

        let mut progress = self.progress.lock().await;
        progress.completed_files += 1;
        progress.downloaded_bytes += task.size.unwrap_or(bytes.len() as u64);

        Ok(())
    }

    pub async fn progress(&self) -> DownloadProgress {
        self.progress.lock().await.clone()
    }
}

async fn verify_sha1(path: &Path, expected: &str) -> Result<bool> {
    use sha1::{Digest, Sha1};

    let data = tokio::fs::read(path).await?;
    let hash = Sha1::digest(&data);
    let hex = format!("{:x}", hash);
    Ok(hex == expected)
}
