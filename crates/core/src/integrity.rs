use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::error::Result;
use crate::version::meta::VersionMeta;

#[derive(Debug, Clone)]
pub struct IntegrityReport {
    pub total_files: usize,
    pub verified: usize,
    pub missing: Vec<DownloadTask>,
    pub corrupted: Vec<DownloadTask>,
}

impl IntegrityReport {
    pub fn is_healthy(&self) -> bool {
        self.missing.is_empty() && self.corrupted.is_empty()
    }

    pub fn repair_tasks(&self) -> Vec<DownloadTask> {
        let mut tasks = self.missing.clone();
        tasks.extend(self.corrupted.iter().cloned());
        tasks
    }
}

pub fn verify_instance_files(mc_version: &str, config: &LauncherConfig) -> Result<IntegrityReport> {
    let meta_path = config
        .versions_dir()
        .join(mc_version)
        .join(format!("{}.json", mc_version));

    if !meta_path.exists() {
        return Err(crate::error::MiaoError::Other(format!(
            "Version meta not found: {}",
            meta_path.display()
        )));
    }

    let meta_content = std::fs::read_to_string(&meta_path)?;
    let meta: VersionMeta = serde_json::from_str(&meta_content)?;

    let mut total_files = 0;
    let mut verified = 0;
    let mut missing = Vec::new();
    let mut corrupted = Vec::new();

    {
        let client = &meta.downloads.client;
        let jar_path = config
            .versions_dir()
            .join(mc_version)
            .join(format!("{}.jar", mc_version));
        total_files += 1;

        match check_file(&jar_path, Some(&client.sha1), Some(client.size)) {
            FileStatus::Ok => verified += 1,
            FileStatus::Missing => {
                missing.push(DownloadTask {
                    url: client.url.clone(),
                    dest: jar_path,
                    sha1: Some(client.sha1.clone()),
                    size: Some(client.size),
                });
            }
            FileStatus::Corrupted => {
                corrupted.push(DownloadTask {
                    url: client.url.clone(),
                    dest: jar_path,
                    sha1: Some(client.sha1.clone()),
                    size: Some(client.size),
                });
            }
        }
    }

    for lib in &meta.libraries {
        if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            let lib_path = config.libraries_dir().join(&artifact.path);
            total_files += 1;

            match check_file(&lib_path, Some(&artifact.sha1), Some(artifact.size)) {
                FileStatus::Ok => verified += 1,
                FileStatus::Missing => {
                    missing.push(DownloadTask {
                        url: artifact.url.clone(),
                        dest: lib_path,
                        sha1: Some(artifact.sha1.clone()),
                        size: Some(artifact.size),
                    });
                }
                FileStatus::Corrupted => {
                    corrupted.push(DownloadTask {
                        url: artifact.url.clone(),
                        dest: lib_path,
                        sha1: Some(artifact.sha1.clone()),
                        size: Some(artifact.size),
                    });
                }
            }
        }
    }

    let asset_index_path = get_asset_index_path(&meta, config);
    if asset_index_path.exists()
        && let Ok(index_content) = std::fs::read_to_string(&asset_index_path)
        && let Ok(index) = serde_json::from_str::<AssetIndex>(&index_content)
    {
        for obj in index.objects.values() {
            let hash_prefix = &obj.hash[..2];
            let asset_path = config
                .assets_dir()
                .join("objects")
                .join(hash_prefix)
                .join(&obj.hash);
            total_files += 1;

            match check_file(&asset_path, Some(&obj.hash), Some(obj.size)) {
                FileStatus::Ok => verified += 1,
                FileStatus::Missing => {
                    let url = format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        hash_prefix, obj.hash
                    );
                    missing.push(DownloadTask {
                        url,
                        dest: asset_path,
                        sha1: Some(obj.hash.clone()),
                        size: Some(obj.size),
                    });
                }
                FileStatus::Corrupted => {
                    let url = format!(
                        "https://resources.download.minecraft.net/{}/{}",
                        hash_prefix, obj.hash
                    );
                    corrupted.push(DownloadTask {
                        url,
                        dest: asset_path,
                        sha1: Some(obj.hash.clone()),
                        size: Some(obj.size),
                    });
                }
            }
        }
    }

    Ok(IntegrityReport {
        total_files,
        verified,
        missing,
        corrupted,
    })
}

enum FileStatus {
    Ok,
    Missing,
    Corrupted,
}

fn check_file(path: &Path, expected_sha1: Option<&str>, expected_size: Option<u64>) -> FileStatus {
    if !path.exists() {
        return FileStatus::Missing;
    }

    if let Some(expected_size) = expected_size
        && let Ok(metadata) = std::fs::metadata(path)
        && metadata.len() != expected_size
    {
        return FileStatus::Corrupted;
    }

    if let Some(expected_sha1) = expected_sha1 {
        if let Ok(data) = std::fs::read(path) {
            let mut hasher = Sha1::new();
            hasher.update(&data);
            let hash = format!("{:x}", hasher.finalize());
            if hash != expected_sha1 {
                return FileStatus::Corrupted;
            }
        } else {
            return FileStatus::Corrupted;
        }
    }

    FileStatus::Ok
}

fn get_asset_index_path(meta: &VersionMeta, config: &LauncherConfig) -> PathBuf {
    config
        .assets_dir()
        .join("indexes")
        .join(format!("{}.json", meta.asset_index.id))
}

#[derive(Debug, serde::Deserialize)]
struct AssetIndex {
    objects: std::collections::HashMap<String, AssetObject>,
}

#[derive(Debug, serde::Deserialize)]
struct AssetObject {
    hash: String,
    size: u64,
}
