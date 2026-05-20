use anyhow::Result;
use serde::Deserialize;

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::http::HttpClient;

const NEOFORGE_MAVEN_URL: &str = "https://maven.neoforged.net/releases";
const NEOFORGE_META_URL: &str =
    "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

#[derive(Debug, Deserialize)]
pub struct NeoForgeVersionList {
    pub versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeoForgeProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<NeoForgeLibrary>,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeLibrary {
    pub name: String,
    pub downloads: Option<NeoForgeLibraryDownloads>,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeLibraryDownloads {
    pub artifact: Option<NeoForgeArtifact>,
}

#[derive(Debug, Deserialize)]
pub struct NeoForgeArtifact {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub async fn fetch_versions(
    http: &impl HttpClient,
    minecraft_version: &str,
) -> Result<Vec<String>> {
    let list: NeoForgeVersionList = http.get_json(NEOFORGE_META_URL).await?;

    let mc_prefix = minecraft_version
        .strip_prefix("1.")
        .unwrap_or(minecraft_version);

    let matching: Vec<String> = list
        .versions
        .into_iter()
        .filter(|v| v.starts_with(mc_prefix))
        .rev()
        .collect();

    Ok(matching)
}

pub async fn fetch_profile(
    http: &impl HttpClient,
    neoforge_version: &str,
) -> Result<NeoForgeProfile> {
    let url = format!(
        "{}/net/neoforged/neoforge/{}/neoforge-{}.json",
        NEOFORGE_MAVEN_URL, neoforge_version, neoforge_version
    );
    http.get_json(&url).await
}

fn maven_to_path(name: &str) -> Option<String> {
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

pub fn collect_library_downloads(
    profile: &NeoForgeProfile,
    config: &LauncherConfig,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    for lib in &profile.libraries {
        if let Some(downloads) = &lib.downloads
            && let Some(artifact) = &downloads.artifact
        {
            if artifact.url.is_empty() {
                continue;
            }
            let dest = config.libraries_dir().join(&artifact.path);
            tasks.push(DownloadTask {
                url: artifact.url.clone(),
                dest,
                sha1: artifact.sha1.clone(),
                size: artifact.size,
            });
        } else {
            let Some(path) = maven_to_path(&lib.name) else {
                continue;
            };
            let url = format!("{}/{}", NEOFORGE_MAVEN_URL, path);
            let dest = config.libraries_dir().join(&path);
            tasks.push(DownloadTask {
                url,
                dest,
                sha1: None,
                size: None,
            });
        }
    }

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_to_path_neoforge() {
        let result = maven_to_path("net.neoforged:neoforge:20.4.80");
        assert_eq!(
            result,
            Some("net/neoforged/neoforge/20.4.80/neoforge-20.4.80.jar".to_string())
        );
    }

    #[test]
    fn maven_to_path_invalid() {
        assert_eq!(maven_to_path("bad"), None);
    }

    #[test]
    fn collect_downloads_with_artifact() {
        let profile = NeoForgeProfile {
            id: "neoforge-20.4.80".to_string(),
            main_class: "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string(),
            libraries: vec![NeoForgeLibrary {
                name: "net.neoforged:neoforge:20.4.80".to_string(),
                downloads: Some(NeoForgeLibraryDownloads {
                    artifact: Some(NeoForgeArtifact {
                        path: "net/neoforged/neoforge/20.4.80/neoforge-20.4.80.jar".to_string(),
                        url: "https://maven.neoforged.net/releases/net/neoforged/neoforge/20.4.80/neoforge-20.4.80.jar".to_string(),
                        sha1: Some("abc123".to_string()),
                        size: Some(5000),
                    }),
                }),
            }],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].url.contains("neoforge-20.4.80.jar"));
        assert_eq!(tasks[0].sha1, Some("abc123".to_string()));
    }

    #[test]
    fn collect_downloads_without_artifact_uses_maven() {
        let profile = NeoForgeProfile {
            id: "neoforge-20.4.80".to_string(),
            main_class: "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string(),
            libraries: vec![NeoForgeLibrary {
                name: "cpw.mods:securejarhandler:2.1.27".to_string(),
                downloads: None,
            }],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);
        assert_eq!(tasks.len(), 1);
        assert!(
            tasks[0]
                .url
                .contains("cpw/mods/securejarhandler/2.1.27/securejarhandler-2.1.27.jar")
        );
    }

    #[test]
    fn collect_downloads_skips_empty_url() {
        let profile = NeoForgeProfile {
            id: "test".to_string(),
            main_class: String::new(),
            libraries: vec![NeoForgeLibrary {
                name: "local:lib:1.0".to_string(),
                downloads: Some(NeoForgeLibraryDownloads {
                    artifact: Some(NeoForgeArtifact {
                        path: "local/lib.jar".to_string(),
                        url: String::new(),
                        sha1: None,
                        size: None,
                    }),
                }),
            }],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);
        assert!(tasks.is_empty());
    }

    #[test]
    fn neoforge_version_list_deserializes() {
        let json = r#"{"versions": ["20.4.80", "20.4.79", "20.3.1"]}"#;
        let list: NeoForgeVersionList = serde_json::from_str(json).unwrap();
        assert_eq!(list.versions.len(), 3);
    }

    #[test]
    fn neoforge_profile_deserializes() {
        let json = r#"{
            "id": "neoforge-20.4.80",
            "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
            "libraries": [
                {"name": "net.neoforged:neoforge:20.4.80", "downloads": {"artifact": {"path": "a.jar", "url": "https://example.com/a.jar", "sha1": "abc", "size": 100}}}
            ]
        }"#;
        let p: NeoForgeProfile = serde_json::from_str(json).unwrap();
        assert_eq!(p.main_class, "cpw.mods.bootstraplauncher.BootstrapLauncher");
    }
}
