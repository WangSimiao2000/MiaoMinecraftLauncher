use anyhow::Result;
use serde::Deserialize;

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::http::HttpClient;

const QUILT_META_URL: &str = "https://meta.quiltmc.org/v3";

#[derive(Debug, Deserialize)]
pub struct QuiltLoaderVersion {
    pub loader: QuiltLoader,
}

#[derive(Debug, Deserialize)]
pub struct QuiltLoader {
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuiltProfile {
    pub id: String,
    pub main_class: String,
    pub libraries: Vec<QuiltLibrary>,
}

#[derive(Debug, Deserialize)]
pub struct QuiltLibrary {
    pub name: String,
    pub url: String,
}

pub async fn fetch_loader_versions(
    http: &impl HttpClient,
    minecraft_version: &str,
) -> Result<Vec<QuiltLoaderVersion>> {
    let url = format!("{}/versions/loader/{}", QUILT_META_URL, minecraft_version);
    http.get_json(&url).await
}

pub async fn fetch_profile(
    http: &impl HttpClient,
    minecraft_version: &str,
    loader_version: &str,
) -> Result<QuiltProfile> {
    let url = format!(
        "{}/versions/loader/{}/{}/profile/json",
        QUILT_META_URL, minecraft_version, loader_version
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
    profile: &QuiltProfile,
    config: &LauncherConfig,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    for lib in &profile.libraries {
        let Some(path) = maven_to_path(&lib.name) else {
            continue;
        };
        let url = format!("{}{}", lib.url, path);
        let dest = config.libraries_dir().join(&path);
        tasks.push(DownloadTask {
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
    fn maven_to_path_basic() {
        let result = maven_to_path("org.quiltmc:quilt-loader:0.23.1");
        assert_eq!(
            result,
            Some("org/quiltmc/quilt-loader/0.23.1/quilt-loader-0.23.1.jar".to_string())
        );
    }

    #[test]
    fn maven_to_path_invalid() {
        assert_eq!(maven_to_path("invalid"), None);
    }

    #[test]
    fn collect_library_downloads_creates_tasks() {
        let profile = QuiltProfile {
            id: "quilt-loader-0.23.1-1.20.4".to_string(),
            main_class: "org.quiltmc.loader.impl.launch.knot.KnotClient".to_string(),
            libraries: vec![
                QuiltLibrary {
                    name: "org.quiltmc:quilt-loader:0.23.1".to_string(),
                    url: "https://maven.quiltmc.org/repository/release/".to_string(),
                },
                QuiltLibrary {
                    name: "org.quiltmc:hashed:1.0.0".to_string(),
                    url: "https://maven.quiltmc.org/repository/release/".to_string(),
                },
            ],
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&profile, &config);

        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].url.contains("quilt-loader"));
        assert!(tasks[0].dest.to_string_lossy().contains("quilt-loader"));
    }

    #[test]
    fn quilt_loader_version_deserializes() {
        let json = r#"{"loader": {"separator": ".", "build": 1, "version": "0.23.1"}}"#;
        let v: QuiltLoaderVersion = serde_json::from_str(json).unwrap();
        assert_eq!(v.loader.version, "0.23.1");
    }

    #[test]
    fn quilt_profile_deserializes() {
        let json = r#"{
            "id": "quilt-loader-0.23.1-1.20.4",
            "mainClass": "org.quiltmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                {"name": "org.quiltmc:quilt-loader:0.23.1", "url": "https://maven.quiltmc.org/repository/release/"}
            ]
        }"#;
        let p: QuiltProfile = serde_json::from_str(json).unwrap();
        assert_eq!(
            p.main_class,
            "org.quiltmc.loader.impl.launch.knot.KnotClient"
        );
        assert_eq!(p.libraries.len(), 1);
    }
}
