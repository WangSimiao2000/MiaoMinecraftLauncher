use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::modloader::ModLoaderType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub minecraft_version: String,
    pub mod_loader: Option<ModLoaderConfig>,
    pub java_path: Option<PathBuf>,
    pub jvm_args: Vec<String>,
    pub game_args: Vec<String>,
    pub memory_max_mb: u32,
    pub memory_min_mb: u32,
    pub resolution: Option<Resolution>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_played: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModLoaderConfig {
    pub loader_type: ModLoaderType,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            name: String::new(),
            minecraft_version: String::new(),
            mod_loader: None,
            java_path: None,
            jvm_args: Vec::new(),
            game_args: Vec::new(),
            memory_max_mb: 4096,
            memory_min_mb: 512,
            resolution: None,
            created_at: chrono::Utc::now(),
            last_played: None,
        }
    }
}

impl Instance {
    pub fn instance_dir(base: &PathBuf, name: &str) -> PathBuf {
        base.join(name)
    }

    pub fn mods_dir(instance_dir: &PathBuf) -> PathBuf {
        instance_dir.join("mods")
    }

    pub fn resourcepacks_dir(instance_dir: &PathBuf) -> PathBuf {
        instance_dir.join("resourcepacks")
    }

    pub fn shaderpacks_dir(instance_dir: &PathBuf) -> PathBuf {
        instance_dir.join("shaderpacks")
    }

    pub fn config_file(instance_dir: &PathBuf) -> PathBuf {
        instance_dir.join("instance.toml")
    }
}
