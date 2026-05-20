pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod quilt;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::http::HttpClient;

pub async fn fetch_all_loader_versions(
    http: &impl HttpClient,
    minecraft_version: &str,
) -> Result<std::collections::HashMap<ModLoaderType, Vec<ModLoaderVersion>>> {
    let (fabric_result, quilt_result, neoforge_result, forge_result) = tokio::join!(
        fabric::fetch_loader_versions(http, minecraft_version),
        quilt::fetch_loader_versions(http, minecraft_version),
        neoforge::fetch_versions(http, minecraft_version),
        forge::fetch_recommended_version(http, minecraft_version),
    );

    let mut result = std::collections::HashMap::new();

    if let Ok(fabric_versions) = fabric_result {
        let versions: Vec<ModLoaderVersion> = fabric_versions
            .into_iter()
            .map(|v| ModLoaderVersion {
                loader_type: ModLoaderType::Fabric,
                version: v.loader.version,
                minecraft_version: minecraft_version.to_string(),
                stable: v.loader.stable,
            })
            .collect();
        if !versions.is_empty() {
            result.insert(ModLoaderType::Fabric, versions);
        }
    }

    if let Ok(quilt_versions) = quilt_result {
        let versions: Vec<ModLoaderVersion> = quilt_versions
            .into_iter()
            .map(|v| ModLoaderVersion {
                loader_type: ModLoaderType::Quilt,
                version: v.loader.version,
                minecraft_version: minecraft_version.to_string(),
                stable: true,
            })
            .collect();
        if !versions.is_empty() {
            result.insert(ModLoaderType::Quilt, versions);
        }
    }

    if let Ok(neoforge_versions) = neoforge_result {
        let versions: Vec<ModLoaderVersion> = neoforge_versions
            .into_iter()
            .map(|v| ModLoaderVersion {
                loader_type: ModLoaderType::NeoForge,
                version: v,
                minecraft_version: minecraft_version.to_string(),
                stable: true,
            })
            .collect();
        if !versions.is_empty() {
            result.insert(ModLoaderType::NeoForge, versions);
        }
    }

    if let Ok(Some(forge_version)) = forge_result {
        let versions = vec![ModLoaderVersion {
            loader_type: ModLoaderType::Forge,
            version: forge_version,
            minecraft_version: minecraft_version.to_string(),
            stable: true,
        }];
        result.insert(ModLoaderType::Forge, versions);
    }

    Ok(result)
}

pub async fn install_loader(
    http: &impl HttpClient,
    loader_type: &ModLoaderType,
    mc_version: &str,
    loader_version: &str,
    config: &crate::config::LauncherConfig,
) -> Result<crate::instance::ModLoaderConfig> {
    use crate::download::manager::DownloadManager;

    let tasks = match loader_type {
        ModLoaderType::Fabric => {
            let profile = fabric::fetch_profile(http, mc_version, loader_version).await?;
            fabric::collect_fabric_library_downloads(&profile, config)
        }
        ModLoaderType::Quilt => {
            let profile = quilt::fetch_profile(http, mc_version, loader_version).await?;
            quilt::collect_library_downloads(&profile, config)
        }
        ModLoaderType::NeoForge => {
            let profile = neoforge::fetch_profile(http, loader_version).await?;
            neoforge::collect_library_downloads(&profile, config)
        }
        ModLoaderType::Forge => {
            let profile =
                forge::fetch_install_profile(http, mc_version, loader_version).await?;
            forge::collect_library_downloads(&profile, config)
        }
    };

    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(tasks).await?;

    Ok(crate::instance::ModLoaderConfig {
        loader_type: loader_type.clone(),
        version: loader_version.to_string(),
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
