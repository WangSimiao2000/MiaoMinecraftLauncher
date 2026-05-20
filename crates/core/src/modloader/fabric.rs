use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::LauncherConfig;

const FABRIC_META_URL: &str = "https://meta.fabricmc.net/v2";

#[derive(Debug, Deserialize)]
pub struct FabricLoaderVersion {
    pub loader: FabricLoader,
}

#[derive(Debug, Deserialize)]
pub struct FabricLoader {
    pub version: String,
    pub stable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FabricProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<FabricLibrary>,
}

#[derive(Debug, Deserialize)]
pub struct FabricLibrary {
    pub name: String,
    pub url: String,
}

pub async fn fetch_loader_versions(
    http: &reqwest::Client,
    minecraft_version: &str,
) -> Result<Vec<FabricLoaderVersion>> {
    let url = format!("{}/versions/loader/{}", FABRIC_META_URL, minecraft_version);
    let versions: Vec<FabricLoaderVersion> = http
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("Failed to fetch Fabric loader versions")?;
    Ok(versions)
}

pub async fn fetch_profile(
    http: &reqwest::Client,
    minecraft_version: &str,
    loader_version: &str,
) -> Result<FabricProfile> {
    let url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        FABRIC_META_URL, minecraft_version, loader_version
    );
    let profile: FabricProfile = http
        .get(&url)
        .send()
        .await?
        .json()
        .await
        .context("Failed to fetch Fabric profile")?;
    Ok(profile)
}

pub fn fabric_library_to_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.splitn(3, ':').collect();
    if parts.len() != 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    Some(format!(
        "{}/{}/{}/{}-{}.jar",
        group, artifact, version, artifact, version
    ))
}

pub fn fabric_library_url(lib: &FabricLibrary) -> Option<String> {
    let path = fabric_library_to_path(&lib.name)?;
    Some(format!("{}{}", lib.url, path))
}

pub fn collect_fabric_library_downloads(
    profile: &FabricProfile,
    config: &LauncherConfig,
) -> Vec<crate::download::DownloadTask> {
    let mut tasks = Vec::new();

    for lib in &profile.libraries {
        let Some(path) = fabric_library_to_path(&lib.name) else {
            continue;
        };
        let Some(url) = fabric_library_url(lib) else {
            continue;
        };

        let dest = config.libraries_dir().join(&path);
        tasks.push(crate::download::DownloadTask {
            url,
            dest,
            sha1: None,
            size: None,
        });
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fabric_library_to_path_basic() {
        let result = fabric_library_to_path("net.fabricmc:fabric-loader:0.15.6");
        assert_eq!(
            result,
            Some("net/fabricmc/fabric-loader/0.15.6/fabric-loader-0.15.6.jar".to_string())
        );
    }

    #[test]
    fn fabric_library_to_path_nested() {
        let result = fabric_library_to_path("org.ow2.asm:asm:9.6");
        assert_eq!(result, Some("org/ow2/asm/asm/9.6/asm-9.6.jar".to_string()));
    }

    #[test]
    fn fabric_library_to_path_invalid() {
        assert_eq!(fabric_library_to_path("invalid"), None);
    }

    #[test]
    fn fabric_library_url_constructs_correctly() {
        let lib = FabricLibrary {
            name: "net.fabricmc:fabric-loader:0.15.6".to_string(),
            url: "https://maven.fabricmc.net/".to_string(),
        };
        let url = fabric_library_url(&lib).unwrap();
        assert_eq!(
            url,
            "https://maven.fabricmc.net/net/fabricmc/fabric-loader/0.15.6/fabric-loader-0.15.6.jar"
        );
    }

    #[test]
    fn collect_fabric_downloads_generates_tasks() {
        let profile = FabricProfile {
            id: "fabric-loader-0.15.6-1.20.4".to_string(),
            main_class: "net.fabricmc.loader.impl.launch.knot.KnotClient".to_string(),
            libraries: vec![
                FabricLibrary {
                    name: "net.fabricmc:fabric-loader:0.15.6".to_string(),
                    url: "https://maven.fabricmc.net/".to_string(),
                },
                FabricLibrary {
                    name: "net.fabricmc:tiny-mappings-parser:0.3.0+build.17".to_string(),
                    url: "https://maven.fabricmc.net/".to_string(),
                },
            ],
        };

        let config = LauncherConfig::default();
        let tasks = collect_fabric_library_downloads(&profile, &config);

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].url.contains("fabric-loader"));
        assert!(tasks[0].dest.to_string_lossy().contains("fabric-loader"));
    }

    #[test]
    fn fabric_loader_version_deserializes() {
        let json = r#"{
            "loader": {
                "separator": ".",
                "build": 1,
                "maven": "net.fabricmc:fabric-loader:0.15.6",
                "version": "0.15.6",
                "stable": true
            }
        }"#;

        let v: FabricLoaderVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.loader.version, "0.15.6");
        assert!(v.loader.stable);
    }

    #[test]
    fn fabric_profile_deserializes() {
        let json = r#"{
            "id": "fabric-loader-0.15.6-1.20.4",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                {
                    "name": "net.fabricmc:fabric-loader:0.15.6",
                    "url": "https://maven.fabricmc.net/"
                }
            ]
        }"#;

        let p: FabricProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.id, "fabric-loader-0.15.6-1.20.4");
        assert_eq!(
            p.main_class,
            "net.fabricmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(p.libraries.len(), 1);
    }
}
