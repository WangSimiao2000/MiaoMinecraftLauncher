use anyhow::Result;
use serde::Deserialize;

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::http::HttpClient;

const FORGE_MAVEN_URL: &str = "https://files.minecraftforge.net/maven";
const FORGE_PROMO_URL: &str =
    "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";

#[derive(Debug, Deserialize)]
pub struct ForgePromotions {
    pub promos: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<ForgeLibrary>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeLibrary {
    pub name: String,
    pub downloads: Option<ForgeLibraryDownloads>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeLibraryDownloads {
    pub artifact: Option<ForgeArtifact>,
}

#[derive(Debug, Deserialize)]
pub struct ForgeArtifact {
    pub path: String,
    pub url: String,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub async fn fetch_recommended_version(
    http: &impl HttpClient,
    minecraft_version: &str,
) -> Result<Option<String>> {
    let promos: ForgePromotions = http.get_json(FORGE_PROMO_URL).await?;

    let recommended_key = format!("{}-recommended", minecraft_version);
    let latest_key = format!("{}-latest", minecraft_version);

    Ok(promos
        .promos
        .get(&recommended_key)
        .or_else(|| promos.promos.get(&latest_key))
        .cloned())
}

pub fn forge_installer_url(minecraft_version: &str, forge_version: &str) -> String {
    format!(
        "{}/net/minecraftforge/forge/{mc}-{fg}/forge-{mc}-{fg}-installer.jar",
        FORGE_MAVEN_URL,
        mc = minecraft_version,
        fg = forge_version
    )
}

pub fn forge_universal_url(minecraft_version: &str, forge_version: &str) -> String {
    format!(
        "{}/net/minecraftforge/forge/{mc}-{fg}/forge-{mc}-{fg}-universal.jar",
        FORGE_MAVEN_URL,
        mc = minecraft_version,
        fg = forge_version
    )
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
    profile: &ForgeProfile,
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
            let url = format!("{}/{}", FORGE_MAVEN_URL, path);
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

pub async fn fetch_install_profile(
    http: &impl HttpClient,
    minecraft_version: &str,
    forge_version: &str,
) -> Result<ForgeProfile> {
    let url = format!(
        "{}/net/minecraftforge/forge/{}-{}/forge-{}-{}.json",
        FORGE_MAVEN_URL, minecraft_version, forge_version, minecraft_version, forge_version
    );
    http.get_json(&url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_installer_url_format() {
        let url = forge_installer_url("1.20.4", "49.0.30");
        assert_eq!(
            url,
            "https://files.minecraftforge.net/maven/net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30-installer.jar"
        );
    }

    #[test]
    fn forge_universal_url_format() {
        let url = forge_universal_url("1.20.4", "49.0.30");
        assert_eq!(
            url,
            "https://files.minecraftforge.net/maven/net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30-universal.jar"
        );
    }

    #[test]
    fn maven_to_path_forge() {
        let result = maven_to_path("net.minecraftforge:forge:1.20.4-49.0.30");
        assert_eq!(
            result,
            Some("net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30.jar".to_string())
        );
    }

    #[test]
    fn collect_downloads_with_artifact() {
        let profile = ForgeProfile {
            id: "forge-1.20.4-49.0.30".to_string(),
            main_class: "cpw.mods.bootstraplauncher.BootstrapLauncher".to_string(),
            libraries: vec![ForgeLibrary {
                name: "net.minecraftforge:forge:1.20.4-49.0.30".to_string(),
                downloads: Some(ForgeLibraryDownloads {
                    artifact: Some(ForgeArtifact {
                        path: "net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30.jar"
                            .to_string(),
                        url: "https://files.minecraftforge.net/maven/net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30.jar".to_string(),
                        sha1: Some("sha1hash".to_string()),
                        size: Some(10000),
                    }),
                }),
            }],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].url.contains("forge-1.20.4-49.0.30.jar"));
    }

    #[test]
    fn collect_downloads_skips_empty_url() {
        let profile = ForgeProfile {
            id: "test".to_string(),
            main_class: String::new(),
            libraries: vec![ForgeLibrary {
                name: "local:lib:1.0".to_string(),
                downloads: Some(ForgeLibraryDownloads {
                    artifact: Some(ForgeArtifact {
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
    fn collect_downloads_maven_fallback() {
        let profile = ForgeProfile {
            id: "test".to_string(),
            main_class: String::new(),
            libraries: vec![ForgeLibrary {
                name: "cpw.mods:securejarhandler:2.1.27".to_string(),
                downloads: None,
            }],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].url.contains("securejarhandler"));
    }

    #[test]
    fn promotions_deserializes() {
        let json = r#"{"promos": {"1.20.4-recommended": "49.0.30", "1.20.4-latest": "49.0.31"}}"#;
        let promos: ForgePromotions = serde_json::from_str(json).unwrap();
        assert_eq!(promos.promos["1.20.4-recommended"], "49.0.30");
    }
}
