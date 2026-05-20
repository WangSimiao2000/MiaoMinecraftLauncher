use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::auth::AuthMethod;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub data_dir: PathBuf,
    pub java_paths: Vec<PathBuf>,
    pub download_mirror: DownloadMirror,
    pub max_concurrent_downloads: usize,
    pub accounts: Vec<AuthMethod>,
    pub active_account_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DownloadMirror {
    #[default]
    Official,
    Bmclapi,
    Custom(String),
}

impl Default for LauncherConfig {
    fn default() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("miao-minecraft-launcher");

        Self {
            data_dir,
            java_paths: Vec::new(),
            download_mirror: DownloadMirror::default(),
            max_concurrent_downloads: 64,
            accounts: Vec::new(),
            active_account_index: None,
        }
    }
}

impl LauncherConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("miao-minecraft-launcher")
            .join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn instances_dir(&self) -> PathBuf {
        self.data_dir.join("instances")
    }

    pub fn versions_dir(&self) -> PathBuf {
        self.data_dir.join("versions")
    }

    pub fn libraries_dir(&self) -> PathBuf {
        self.data_dir.join("libraries")
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir.join("assets")
    }
}
