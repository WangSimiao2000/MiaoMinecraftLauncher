pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod optifine;
pub mod quilt;

use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::http::HttpClient;

pub struct ResolvedProfile {
    pub main_class: String,
    pub library_paths: Vec<String>,
    pub download_tasks: Vec<DownloadTask>,
}

pub async fn resolve_loader_profile(
    http: &impl HttpClient,
    loader_type: &ModLoaderType,
    mc_version: &str,
    loader_version: &str,
    config: &LauncherConfig,
) -> Result<ResolvedProfile> {
    match loader_type {
        ModLoaderType::Fabric => {
            let profile = fabric::fetch_profile(http, mc_version, loader_version).await?;
            let library_paths = profile
                .libraries
                .iter()
                .filter_map(|l| {
                    fabric::fabric_library_to_path(&l.name)
                        .map(|p| config.libraries_dir().join(p).to_string_lossy().to_string())
                })
                .collect();
            let tasks = fabric::collect_fabric_library_downloads(&profile, config);
            Ok(ResolvedProfile {
                main_class: profile.main_class,
                library_paths,
                download_tasks: tasks,
            })
        }
        ModLoaderType::Quilt => {
            let profile = quilt::fetch_profile(http, mc_version, loader_version).await?;
            let library_paths = profile
                .libraries
                .iter()
                .filter_map(|l| {
                    fabric::fabric_library_to_path(&l.name)
                        .map(|p| config.libraries_dir().join(p).to_string_lossy().to_string())
                })
                .collect();
            let tasks = quilt::collect_library_downloads(&profile, config);
            Ok(ResolvedProfile {
                main_class: profile.main_class,
                library_paths,
                download_tasks: tasks,
            })
        }
        ModLoaderType::NeoForge => {
            let profile = neoforge::fetch_profile(http, loader_version).await?;
            let library_paths = profile
                .libraries
                .iter()
                .filter_map(|l| {
                    fabric::fabric_library_to_path(&l.name)
                        .map(|p| config.libraries_dir().join(p).to_string_lossy().to_string())
                })
                .collect();
            let tasks = neoforge::collect_library_downloads(&profile, config);
            Ok(ResolvedProfile {
                main_class: profile.main_class,
                library_paths,
                download_tasks: tasks,
            })
        }
        ModLoaderType::Forge => {
            let profile = forge::fetch_install_profile(http, mc_version, loader_version).await?;
            let library_paths = profile
                .libraries
                .iter()
                .filter_map(|l| {
                    fabric::fabric_library_to_path(&l.name)
                        .map(|p| config.libraries_dir().join(p).to_string_lossy().to_string())
                })
                .collect();
            let tasks = forge::collect_library_downloads(&profile, config);
            Ok(ResolvedProfile {
                main_class: profile.main_class,
                library_paths,
                download_tasks: tasks,
            })
        }
    }
}

pub struct LoaderFetchResult {
    pub versions: std::collections::HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    pub failed: Vec<ModLoaderType>,
}

pub async fn fetch_all_loader_versions(
    http: &impl HttpClient,
    minecraft_version: &str,
) -> Result<LoaderFetchResult> {
    let (fabric_result, quilt_result, neoforge_result, forge_result) = tokio::join!(
        fabric::fetch_loader_versions(http, minecraft_version),
        quilt::fetch_loader_versions(http, minecraft_version),
        neoforge::fetch_versions(http, minecraft_version),
        forge::fetch_recommended_version(http, minecraft_version),
    );

    let mut versions = std::collections::HashMap::new();
    let mut failed = Vec::new();

    match fabric_result {
        Ok(fabric_versions) => {
            let v: Vec<ModLoaderVersion> = fabric_versions
                .into_iter()
                .map(|v| ModLoaderVersion {
                    loader_type: ModLoaderType::Fabric,
                    version: v.loader.version,
                    minecraft_version: minecraft_version.to_string(),
                    stable: v.loader.stable,
                })
                .collect();
            if !v.is_empty() {
                versions.insert(ModLoaderType::Fabric, v);
            }
        }
        Err(_) => failed.push(ModLoaderType::Fabric),
    }

    match quilt_result {
        Ok(quilt_versions) => {
            let v: Vec<ModLoaderVersion> = quilt_versions
                .into_iter()
                .map(|v| ModLoaderVersion {
                    loader_type: ModLoaderType::Quilt,
                    version: v.loader.version,
                    minecraft_version: minecraft_version.to_string(),
                    stable: true,
                })
                .collect();
            if !v.is_empty() {
                versions.insert(ModLoaderType::Quilt, v);
            }
        }
        Err(_) => failed.push(ModLoaderType::Quilt),
    }

    match neoforge_result {
        Ok(neoforge_versions) => {
            let v: Vec<ModLoaderVersion> = neoforge_versions
                .into_iter()
                .map(|v| ModLoaderVersion {
                    loader_type: ModLoaderType::NeoForge,
                    version: v,
                    minecraft_version: minecraft_version.to_string(),
                    stable: true,
                })
                .collect();
            if !v.is_empty() {
                versions.insert(ModLoaderType::NeoForge, v);
            }
        }
        Err(_) => failed.push(ModLoaderType::NeoForge),
    }

    match forge_result {
        Ok(Some(forge_version)) => {
            let v = vec![ModLoaderVersion {
                loader_type: ModLoaderType::Forge,
                version: forge_version,
                minecraft_version: minecraft_version.to_string(),
                stable: true,
            }];
            versions.insert(ModLoaderType::Forge, v);
        }
        Ok(None) => {}
        Err(_) => failed.push(ModLoaderType::Forge),
    }

    Ok(LoaderFetchResult { versions, failed })
}

pub async fn install_loader(
    http: &impl HttpClient,
    loader_type: &ModLoaderType,
    mc_version: &str,
    loader_version: &str,
    config: &crate::config::LauncherConfig,
) -> Result<crate::instance::ModLoaderConfig> {
    use crate::download::manager::DownloadManager;

    let resolved =
        resolve_loader_profile(http, loader_type, mc_version, loader_version, config).await?;

    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(resolved.download_tasks).await?;

    Ok(crate::instance::ModLoaderConfig {
        loader_type: loader_type.clone(),
        version: loader_version.to_string(),
        main_class: Some(resolved.main_class),
        extra_libraries: resolved.library_paths,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModLoaderType {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
}

impl ModLoaderType {
    pub const ALL: [ModLoaderType; 4] = [
        ModLoaderType::Fabric,
        ModLoaderType::Quilt,
        ModLoaderType::NeoForge,
        ModLoaderType::Forge,
    ];

    pub fn from_index(idx: usize) -> Option<Self> {
        Self::ALL.get(idx).cloned()
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "fabric" => Ok(Self::Fabric),
            "quilt" => Ok(Self::Quilt),
            "neoforge" => Ok(Self::NeoForge),
            "forge" => Ok(Self::Forge),
            _ => Err(crate::error::MiaoError::Other(format!(
                "Unknown loader '{}'. Use: fabric, quilt, neoforge, forge",
                s
            ))),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::NeoForge => "neoforge",
            Self::Forge => "forge",
        }
    }
}

impl std::fmt::Display for ModLoaderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forge => write!(f, "Forge"),
            Self::NeoForge => write!(f, "NeoForge"),
            Self::Fabric => write!(f, "Fabric"),
            Self::Quilt => write!(f, "Quilt"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLoaderVersion {
    pub loader_type: ModLoaderType,
    pub version: String,
    pub minecraft_version: String,
    pub stable: bool,
}

impl ModLoaderVersion {
    pub fn id(&self) -> String {
        format!(
            "{}-{}-{}",
            self.loader_type, self.minecraft_version, self.version
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modloader_type_display() {
        assert_eq!(ModLoaderType::Forge.to_string(), "Forge");
        assert_eq!(ModLoaderType::NeoForge.to_string(), "NeoForge");
        assert_eq!(ModLoaderType::Fabric.to_string(), "Fabric");
        assert_eq!(ModLoaderType::Quilt.to_string(), "Quilt");
    }

    #[test]
    fn modloader_type_equality() {
        assert_eq!(ModLoaderType::Forge, ModLoaderType::Forge);
        assert_ne!(ModLoaderType::Forge, ModLoaderType::Fabric);
    }

    #[test]
    fn modloader_type_serialization() {
        let loader = ModLoaderType::Fabric;
        let json = serde_json::to_string(&loader).unwrap();
        let deserialized: ModLoaderType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ModLoaderType::Fabric);
    }

    #[test]
    fn modloader_version_id() {
        let v = ModLoaderVersion {
            loader_type: ModLoaderType::Fabric,
            version: "0.15.6".to_string(),
            minecraft_version: "1.20.4".to_string(),
            stable: true,
        };
        assert_eq!(v.id(), "Fabric-1.20.4-0.15.6");
    }

    #[test]
    fn modloader_version_serialization() {
        let v = ModLoaderVersion {
            loader_type: ModLoaderType::NeoForge,
            version: "20.4.80".to_string(),
            minecraft_version: "1.20.4".to_string(),
            stable: true,
        };
        let json = serde_json::to_string(&v).unwrap();
        let deserialized: ModLoaderVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.loader_type, ModLoaderType::NeoForge);
        assert_eq!(deserialized.version, "20.4.80");
        assert!(deserialized.stable);
    }
}
