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

pub async fn fetch_latest_asset(
    http: &reqwest::Client,
    major_version: u32,
) -> Result<AdoptiumAsset> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        other => other,
    };

    let url = format!(
        "{}/assets/latest/{}/hotspot?architecture={}&image_type=jre&os=linux",
        ADOPTIUM_API, major_version, arch
    );

    let assets: Vec<AdoptiumAsset> = http.get(&url).send().await?.json().await?;

    assets.into_iter().next().ok_or_else(|| {
        crate::error::MiaoError::Other(format!(
            "No JRE available for Java {} on {}",
            major_version, arch
        ))
    })
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

    let dest_dir = java_base_dir.join(format!("java-{}", asset.version.major));
    if dest_dir.exists() {
        std::fs::remove_dir_all(&dest_dir)?;
    }
    std::fs::create_dir_all(&dest_dir)?;

    let response = http
        .get(&asset.binary.package.link)
        .send()
        .await?
        .error_for_status()?;

    let total_size = asset.binary.package.size;
    let mut downloaded: u64 = 0;
    let mut all_bytes = Vec::with_capacity(total_size as usize);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        all_bytes.extend_from_slice(&chunk);
        on_progress(DownloadPhase::Downloading {
            downloaded,
            total: total_size,
        });
    }

    on_progress(DownloadPhase::Extracting);

    let extract_dir = dest_dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let tar_gz = flate2::read::GzDecoder::new(&all_bytes[..]);
        let mut archive = tar::Archive::new(tar_gz);
        archive.unpack(&extract_dir)?;
        Ok(())
    })
    .await??;

    let java_bin = find_java_binary(&dest_dir)?;
    Ok(java_bin)
}

fn find_java_binary(extracted_dir: &Path) -> Result<PathBuf> {
    for entry in std::fs::read_dir(extracted_dir)?.flatten() {
        let bin_path = entry.path().join("bin/java");
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
