use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModLoaderType {
    Forge,
    NeoForge,
    Fabric,
    Quilt,
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
        format!("{}-{}-{}", self.loader_type, self.minecraft_version, self.version)
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
