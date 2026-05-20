use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

use crate::config::{DownloadMirror, LauncherConfig};
use crate::download::mirror::transform_url;
use crate::download::DownloadTask;

const MOJANG_RESOURCES_BASE: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Deserialize)]
pub struct AssetIndexFile {
    pub objects: HashMap<String, AssetObject>,
}

#[derive(Debug, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

pub async fn fetch_asset_index(
    index_path: &std::path::Path,
) -> Result<AssetIndexFile> {
    let content = tokio::fs::read_to_string(index_path).await?;
    let index: AssetIndexFile = serde_json::from_str(&content)?;
    Ok(index)
}

pub fn collect_asset_downloads(
    index: &AssetIndexFile,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    for obj in index.objects.values() {
        let hash_prefix = &obj.hash[..2];
        let url = format!("{}/{}/{}", MOJANG_RESOURCES_BASE, hash_prefix, obj.hash);
        let dest = config
            .assets_dir()
            .join("objects")
            .join(hash_prefix)
            .join(&obj.hash);

        tasks.push(DownloadTask {
            url: transform_url(&url, mirror),
            dest,
            sha1: Some(obj.hash.clone()),
            size: Some(obj.size),
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_asset_downloads_generates_correct_paths() {
        let mut objects = HashMap::new();
        objects.insert(
            "minecraft/sounds/ambient/cave/cave1.ogg".to_string(),
            AssetObject {
                hash: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
                size: 12345,
            },
        );

        let index = AssetIndexFile { objects };
        let config = LauncherConfig::default();
        let tasks = collect_asset_downloads(&index, &config, &DownloadMirror::Official);

        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert_eq!(
            task.url,
            "https://resources.download.minecraft.net/ab/abcdef1234567890abcdef1234567890abcdef12"
        );
        assert!(task.dest.ends_with("objects/ab/abcdef1234567890abcdef1234567890abcdef12"));
        assert_eq!(task.sha1.as_deref(), Some("abcdef1234567890abcdef1234567890abcdef12"));
        assert_eq!(task.size, Some(12345));
    }

    #[test]
    fn collect_asset_downloads_with_bmclapi() {
        let mut objects = HashMap::new();
        objects.insert(
            "test.ogg".to_string(),
            AssetObject {
                hash: "ff0011223344556677889900aabbccddeeff0011".to_string(),
                size: 100,
            },
        );

        let index = AssetIndexFile { objects };
        let config = LauncherConfig::default();
        let tasks = collect_asset_downloads(&index, &config, &DownloadMirror::Bmclapi);

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].url.contains("bmclapi2.bangbang93.com/assets/ff/"));
    }

    #[test]
    fn parse_asset_index_json() {
        let json = r#"{
            "objects": {
                "icons/icon_16x16.png": {
                    "hash": "bdf48ef6b5d0d23bbb02907e5522656918942e4e",
                    "size": 3665
                },
                "icons/icon_32x32.png": {
                    "hash": "92750c5f93c312ba9ab413d546f32190c56d6f1f",
                    "size": 5362
                }
            }
        }"#;

        let index: AssetIndexFile = serde_json::from_str(json).unwrap();
        assert_eq!(index.objects.len(), 2);
        assert_eq!(
            index.objects["icons/icon_16x16.png"].hash,
            "bdf48ef6b5d0d23bbb02907e5522656918942e4e"
        );
        assert_eq!(index.objects["icons/icon_16x16.png"].size, 3665);
    }
}
