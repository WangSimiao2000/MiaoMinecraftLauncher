use std::path::{Path, PathBuf};

use crate::error::Result;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum DownloadPhase {
    Downloading { downloaded: u64, total: u64 },
    Extracting,
}

const ADOPTIUM_API: &str = "https://api.adoptium.net/v3";

#[derive(Debug, Deserialize)]
pub struct AdoptiumAsset {
    pub binary: AdoptiumBinary,
    pub release_name: String,
    pub version: AdoptiumVersion,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumBinary {
    pub architecture: String,
    pub image_type: String,
    pub os: String,
    pub package: AdoptiumPackage,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumPackage {
    pub checksum: String,
    pub link: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumVersion {
    pub major: u32,
    pub minor: u32,
    pub security: u32,
    pub openjdk_version: String,
    pub semver: String,
}

#[derive(Debug, Deserialize)]
pub struct AvailableReleases {
    pub available_releases: Vec<u32>,
    pub available_lts_releases: Vec<u32>,
}

pub async fn fetch_available_releases(http: &reqwest::Client) -> Result<AvailableReleases> {
    let url = format!("{}/info/available_releases", ADOPTIUM_API);
    let releases: AvailableReleases = http.get(&url).send().await?.json().await?;
    Ok(releases)
}

pub fn adoptium_os() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "mac",
        "windows" => "windows",
        other => other,
    }
}

pub async fn fetch_latest_asset(
    http: &reqwest::Client,
    major_version: u32,
) -> Result<AdoptiumAsset> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => other,
    };

    let os = adoptium_os();

    let url = format!(
        "{}/assets/latest/{}/hotspot?architecture={}&image_type=jre&os={}",
        ADOPTIUM_API, major_version, arch, os
    );

    // 添加重试机制
    let mut attempts = 0;
    let max_attempts = 3;
    let mut last_error = None;

    while attempts < max_attempts {
        attempts += 1;

        match http
            .get(&url)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(response) => {
                match response.json::<Vec<AdoptiumAsset>>().await {
                    Ok(assets) => {
                        if let Some(asset) = assets.into_iter().next() {
                            // 验证下载链接
                            if asset.binary.package.link.is_empty() {
                                last_error = Some(crate::error::MiaoError::Other(
                                    "Empty download link in API response".to_string(),
                                ));
                                continue;
                            }
                            return Ok(asset);
                        } else {
                            last_error = Some(crate::error::MiaoError::Other(format!(
                                "No JRE available for Java {} on {}/{}",
                                major_version, os, arch
                            )));
                        }
                    }
                    Err(e) => {
                        last_error = Some(e.into());
                        if attempts < max_attempts {
                            tokio::time::sleep(std::time::Duration::from_secs(2 * attempts)).await;
                            continue;
                        }
                    }
                }
            }
            Err(e) => {
                last_error = Some(e.into());
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(2 * attempts)).await;
                    continue;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        crate::error::MiaoError::Other(format!(
            "Failed to fetch Java {} asset after {} attempts",
            major_version, max_attempts
        ))
    }))
}

pub async fn download_and_extract_java(
    http: &reqwest::Client,
    asset: &AdoptiumAsset,
    java_base_dir: &Path,
) -> Result<PathBuf> {
    download_and_extract_java_with_progress(http, asset, java_base_dir, |_| {}).await
}

pub async fn download_and_extract_java_with_progress(
    http: &reqwest::Client,
    asset: &AdoptiumAsset,
    java_base_dir: &Path,
    on_progress: impl Fn(DownloadPhase),
) -> Result<PathBuf> {
    use futures::StreamExt;
    use std::io::Write;

    let dest_dir = java_base_dir.join(format!("java-{}", asset.version.major));
    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir)?;
    }
    std::fs::create_dir_all(&dest_dir)?;

    // 创建临时文件来存储下载
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("java-download-{}.tmp", std::process::id()));

    // 添加超时和重试
    let mut attempts = 0;
    let max_attempts = 3;
    let mut last_error = None;
    let mut response_opt = None;

    while attempts < max_attempts {
        attempts += 1;

        match http
            .get(&asset.binary.package.link)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
        {
            Ok(response) => match response.error_for_status() {
                Ok(response) => {
                    response_opt = Some(response);
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempts < max_attempts {
                        tokio::time::sleep(std::time::Duration::from_secs(2 * attempts)).await;
                        continue;
                    }
                }
            },
            Err(e) => {
                last_error = Some(e);
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(2 * attempts)).await;
                    continue;
                }
            }
        }
    }

    // 如果所有尝试都失败
    let response = match response_opt {
        Some(response) => response,
        None => {
            if let Some(err) = last_error {
                return Err(err.into());
            } else {
                return Err(crate::error::MiaoError::Other(
                    "Failed to download Java after multiple attempts".to_string(),
                ));
            }
        }
    };

    let total_size = asset.binary.package.size;
    let mut downloaded: u64 = 0;

    // 流式下载到文件，避免内存占用过大
    let mut file = std::fs::File::create(&temp_path)?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        file.write_all(&chunk)?;

        on_progress(DownloadPhase::Downloading {
            downloaded,
            total: total_size,
        });
    }

    // 确保所有数据都写入磁盘
    file.sync_all()?;
    drop(file); // 关闭文件句柄

    on_progress(DownloadPhase::Extracting);

    let extract_dir = dest_dir.clone();
    let is_windows = std::env::consts::OS == "windows";
    tokio::task::spawn_blocking(move || -> Result<()> {
        if is_windows {
            let file = std::fs::File::open(&temp_path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(&extract_dir)?;
        } else {
            let file = std::fs::File::open(&temp_path)?;
            let tar_gz = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(tar_gz);
            archive.unpack(&extract_dir)?;
        }

        // 清理临时文件
        let _ = std::fs::remove_file(&temp_path);
        Ok(())
    })
    .await??;

    let java_bin = find_java_binary(&dest_dir)?;
    Ok(java_bin)
}

fn find_java_binary(extracted_dir: &Path) -> Result<PathBuf> {
    let binary_name = super::java_binary_name();
    for entry in std::fs::read_dir(extracted_dir)?.flatten() {
        let bin_path = entry.path().join("bin").join(binary_name);
        if bin_path.exists() {
            return Ok(bin_path);
        }
    }
    Err(crate::error::MiaoError::Other(
        "Could not find java binary in extracted archive".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arch_mapping() {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "aarch64",
            other => other,
        };
        assert!(!arch.is_empty());
    }

    #[test]
    fn available_releases_deserializes() {
        let json = r#"{"available_releases":[8,11,17,21],"available_lts_releases":[8,11,17,21],"most_recent_feature_release":21,"most_recent_feature_version":22,"most_recent_lts":21,"tip_version":22}"#;
        let releases: AvailableReleases = serde_json::from_str(json).unwrap();
        assert_eq!(releases.available_releases, vec![8, 11, 17, 21]);
        assert_eq!(releases.available_lts_releases, vec![8, 11, 17, 21]);
    }

    #[test]
    fn adoptium_asset_deserializes() {
        let json = r#"{
            "binary": {
                "architecture": "x64",
                "download_count": 100,
                "heap_size": "normal",
                "image_type": "jre",
                "jvm_impl": "hotspot",
                "os": "linux",
                "package": {
                    "checksum": "abc123",
                    "checksum_link": "https://example.com/sha256",
                    "download_count": 100,
                    "link": "https://example.com/jre.tar.gz",
                    "metadata_link": "https://example.com/meta.json",
                    "name": "OpenJDK21U-jre_x64_linux_hotspot_21.0.3_9.tar.gz",
                    "signature_link": "https://example.com/sig",
                    "size": 52000000
                },
                "project": "jdk",
                "scm_ref": "jdk-21.0.3+9",
                "updated_at": "2024-01-01T00:00:00Z"
            },
            "release_link": "https://example.com/release",
            "release_name": "jdk-21.0.3+9",
            "vendor": "eclipse",
            "version": {
                "build": 9,
                "major": 21,
                "minor": 0,
                "openjdk_version": "21.0.3+9-LTS",
                "optional": "LTS",
                "security": 3,
                "semver": "21.0.3+9.0.LTS"
            }
        }"#;

        let asset: AdoptiumAsset = serde_json::from_str(json).unwrap();
        assert_eq!(asset.version.major, 21);
        assert_eq!(asset.binary.package.size, 52000000);
        assert_eq!(asset.binary.architecture, "x64");
    }
}
