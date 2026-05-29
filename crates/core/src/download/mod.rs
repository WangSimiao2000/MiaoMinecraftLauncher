pub mod manager;
pub mod mirror;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub url: String,
    pub dest: std::path::PathBuf,
    pub sha1: Option<String>,
    /// Lowercase-hex SHA-256. When both `sha1` and `sha256` are set, both are verified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub total_files: usize,
    pub completed_files: usize,
    pub current_file: String,
}
