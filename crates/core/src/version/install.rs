use crate::error::Result;

use crate::config::{DownloadMirror, LauncherConfig};
use crate::download::DownloadTask;
use crate::download::mirror::transform_url;
use crate::http::HttpClient;

use super::meta::VersionMeta;

pub async fn fetch_version_meta(
    http: &impl HttpClient,
    version_url: &str,
    mirror: &DownloadMirror,
) -> Result<VersionMeta> {
    let url = transform_url(version_url, mirror);
    http.get_json(&url).await
}

pub fn collect_library_downloads(
    meta: &VersionMeta,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();

    for lib in &meta.libraries {
        if !VersionMeta::is_library_allowed(lib) {
            continue;
        }

        if let Some(downloads) = &lib.downloads
            && let Some(artifact) = &downloads.artifact
        {
            let dest = config.libraries_dir().join(&artifact.path);
            tasks.push(DownloadTask {
                url: transform_url(&artifact.url, mirror),
                dest,
                sha1: Some(artifact.sha1.clone()),
                size: Some(artifact.size),
            });
        }
    }

    tasks
}

pub fn collect_client_download(
    meta: &VersionMeta,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> DownloadTask {
    let dest = config
        .versions_dir()
        .join(&meta.id)
        .join(format!("{}.jar", meta.id));

    DownloadTask {
        url: transform_url(&meta.downloads.client.url, mirror),
        dest,
        sha1: Some(meta.downloads.client.sha1.clone()),
        size: Some(meta.downloads.client.size),
    }
}

pub fn collect_asset_index_download(
    meta: &VersionMeta,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> DownloadTask {
    let dest = config
        .assets_dir()
        .join("indexes")
        .join(format!("{}.json", meta.asset_index.id));

    DownloadTask {
        url: transform_url(&meta.asset_index.url, mirror),
        dest,
        sha1: Some(meta.asset_index.sha1.clone()),
        size: Some(meta.asset_index.size),
    }
}

pub fn save_version_meta(meta: &VersionMeta, config: &LauncherConfig) -> Result<()> {
    let dir = config.versions_dir().join(&meta.id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", meta.id));
    let content = serde_json::to_string_pretty(meta)?;
    std::fs::write(&path, content)?;
    Ok(())
}

pub fn all_download_tasks(
    meta: &VersionMeta,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> Vec<DownloadTask> {
    let mut tasks = collect_library_downloads(meta, config, mirror);
    tasks.push(collect_client_download(meta, config, mirror));
    tasks.push(collect_asset_index_download(meta, config, mirror));
    tasks
}

pub fn natives_dir(config: &LauncherConfig, version_id: &str) -> std::path::PathBuf {
    config.versions_dir().join(version_id).join("natives")
}

fn current_os_natives_key() -> &'static str {
    match std::env::consts::OS {
        "linux" => "linux",
        "windows" => "windows",
        "macos" => "osx",
        _ => "linux",
    }
}

pub fn collect_native_downloads(
    meta: &VersionMeta,
    config: &LauncherConfig,
    mirror: &DownloadMirror,
) -> Vec<DownloadTask> {
    let mut tasks = Vec::new();
    let os_key = current_os_natives_key();

    for lib in &meta.libraries {
        if !VersionMeta::is_library_allowed(lib) {
            continue;
        }

        let Some(ref natives_map) = lib.natives else {
            continue;
        };
        let Some(classifier_key) = natives_map.get(os_key) else {
            continue;
        };

        let classifier_key = classifier_key
            .replace("${arch}", std::env::consts::ARCH)
            .replace("${os.arch}", std::env::consts::ARCH);

        if let Some(ref downloads) = lib.downloads
            && let Some(ref classifiers) = downloads.classifiers
            && let Some(artifact) = classifiers.get(&classifier_key)
        {
            let dest = config.libraries_dir().join(&artifact.path);
            tasks.push(DownloadTask {
                url: transform_url(&artifact.url, mirror),
                dest,
                sha1: Some(artifact.sha1.clone()),
                size: Some(artifact.size),
            });
        }
    }

    tasks
}

pub fn extract_natives(meta: &VersionMeta, config: &LauncherConfig) -> Result<()> {
    let dest_dir = natives_dir(config, &meta.id);
    std::fs::create_dir_all(&dest_dir)?;

    let os_key = current_os_natives_key();

    for lib in &meta.libraries {
        if !VersionMeta::is_library_allowed(lib) {
            continue;
        }

        let Some(ref natives_map) = lib.natives else {
            continue;
        };
        let Some(classifier_key) = natives_map.get(os_key) else {
            continue;
        };

        let classifier_key = classifier_key
            .replace("${arch}", std::env::consts::ARCH)
            .replace("${os.arch}", std::env::consts::ARCH);

        if let Some(ref downloads) = lib.downloads
            && let Some(ref classifiers) = downloads.classifiers
            && let Some(artifact) = classifiers.get(&classifier_key)
        {
            let jar_path = config.libraries_dir().join(&artifact.path);
            if !jar_path.exists() {
                continue;
            }

            let exclude = lib
                .extract
                .as_ref()
                .map(|e| e.exclude.as_slice())
                .unwrap_or(&[]);

            let file = std::fs::File::open(&jar_path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut entry = archive.by_index(i)?;
                let name = entry.name().to_string();

                if entry.is_dir() {
                    continue;
                }

                let should_exclude = exclude.iter().any(|ex| name.starts_with(ex));
                if should_exclude {
                    continue;
                }

                let out_path = dest_dir.join(&name);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = std::fs::File::create(&out_path)?;
                std::io::copy(&mut entry, &mut out_file)?;
            }
        }
    }

    Ok(())
}

pub fn library_maven_path(name: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_maven_path_basic() {
        let result = library_maven_path("com.mojang:authlib:3.16.29");
        assert_eq!(
            result,
            Some("com/mojang/authlib/3.16.29/authlib-3.16.29.jar".to_string())
        );
    }

    #[test]
    fn library_maven_path_nested_group() {
        let result = library_maven_path("org.apache.logging.log4j:log4j-api:2.19.0");
        assert_eq!(
            result,
            Some("org/apache/logging/log4j/log4j-api/2.19.0/log4j-api-2.19.0.jar".to_string())
        );
    }

    #[test]
    fn library_maven_path_invalid() {
        assert_eq!(library_maven_path("invalid"), None);
        assert_eq!(library_maven_path("only:two"), None);
    }

    #[test]
    fn collect_library_downloads_filters_non_linux() {
        use crate::version::meta::*;

        let meta = VersionMeta {
            id: "1.20.4".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![
                Library {
                    name: "com.mojang:authlib:3.16.29".to_string(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(Artifact {
                            path: "com/mojang/authlib/3.16.29/authlib-3.16.29.jar".to_string(),
                            sha1: "abc123".to_string(),
                            size: 1024,
                            url: "https://libraries.minecraft.net/com/mojang/authlib/3.16.29/authlib-3.16.29.jar".to_string(),
                        }),
                        classifiers: None,
                    }),
                    rules: None,
                    natives: None,
                    extract: None,
                },
                Library {
                    name: "org.lwjgl:lwjgl:3.3.2".to_string(),
                    downloads: Some(LibraryDownloads {
                        artifact: Some(Artifact {
                            path: "org/lwjgl/lwjgl/3.3.2/lwjgl-3.3.2.jar".to_string(),
                            sha1: "def456".to_string(),
                            size: 2048,
                            url: "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.2/lwjgl-3.3.2.jar".to_string(),
                        }),
                        classifiers: None,
                    }),
                    rules: Some(vec![Rule {
                        action: "allow".to_string(),
                        os: Some(OsRule {
                            name: Some("windows".to_string()),
                        }),
                    }]),
                    natives: None,
                    extract: None,
                },
            ],
            asset_index: AssetIndex {
                id: "5".to_string(),
                sha1: "aaa".to_string(),
                size: 100,
                url: "https://piston-meta.mojang.com/v1/packages/aaa/5.json".to_string(),
                total_size: Some(500000),
            },
            downloads: Downloads {
                client: DownloadEntry {
                    sha1: "clientsha".to_string(),
                    size: 25000000,
                    url: "https://piston-data.mojang.com/client.jar".to_string(),
                },
                server: None,
            },
            java_version: Some(JavaVersion { major_version: 17 }),
        };

        let config = LauncherConfig::default();
        let tasks = collect_library_downloads(&meta, &config, &DownloadMirror::Official);

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].url.contains("authlib"));
    }

    #[test]
    fn collect_client_download_correct_path() {
        use crate::version::meta::*;

        let meta = VersionMeta {
            id: "1.20.4".to_string(),
            main_class: "net.minecraft.client.main.Main".to_string(),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![],
            asset_index: AssetIndex {
                id: "5".to_string(),
                sha1: "aaa".to_string(),
                size: 100,
                url: "https://example.com/5.json".to_string(),
                total_size: None,
            },
            downloads: Downloads {
                client: DownloadEntry {
                    sha1: "clientsha1".to_string(),
                    size: 25000000,
                    url: "https://piston-data.mojang.com/v1/objects/abc/client.jar".to_string(),
                },
                server: None,
            },
            java_version: Some(JavaVersion { major_version: 17 }),
        };

        let config = LauncherConfig::default();
        let task = collect_client_download(&meta, &config, &DownloadMirror::Official);

        assert!(task.dest.ends_with("versions/1.20.4/1.20.4.jar"));
        assert_eq!(task.sha1, Some("clientsha1".to_string()));
    }
}
