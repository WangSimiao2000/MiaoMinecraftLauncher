use std::path::PathBuf;

use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::auth::AuthMethod;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub data_dir: PathBuf,
    #[serde(default)]
    pub game_folders: Vec<PathBuf>,
    pub java_paths: Vec<PathBuf>,
    pub download_mirror: DownloadMirror,
    pub max_concurrent_downloads: usize,
    pub accounts: Vec<AuthMethod>,
    pub active_account_index: Option<usize>,
    #[serde(default)]
    pub curseforge_api_key: Option<String>,
    #[serde(default)]
    pub theme: ThemePreset,
    #[serde(default)]
    pub language: LanguagePref,
    #[serde(skip)]
    pub config_file_override: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreset {
    #[default]
    Dark,
    Ocean,
    Forest,
    Warm,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 4] = [
        ThemePreset::Dark,
        ThemePreset::Ocean,
        ThemePreset::Forest,
        ThemePreset::Warm,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::Dark => "Dark (Default)",
            ThemePreset::Ocean => "Ocean Blue",
            ThemePreset::Forest => "Forest Green",
            ThemePreset::Warm => "Warm Amber",
        }
    }
}

/// User language preference. Lives in core only as a serializable enum so the
/// config file format is stable; the actual translation table is in the GUI
/// crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LanguagePref {
    #[default]
    English,
    Chinese,
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
        let data_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.canonicalize().ok())
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .filter(|d| !d.starts_with("/tmp"))
            .map(|d| d.join("mmcl-data"))
            .unwrap_or_else(|| {
                dirs::data_dir()
                    .unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                    })
                    .join("miao-minecraft-launcher")
            });

        Self {
            data_dir,
            game_folders: Vec::new(),
            java_paths: Vec::new(),
            download_mirror: DownloadMirror::default(),
            max_concurrent_downloads: 64,
            accounts: Vec::new(),
            active_account_index: None,
            curseforge_api_key: None,
            theme: ThemePreset::default(),
            language: LanguagePref::default(),
            config_file_override: None,
        }
    }
}

impl LauncherConfig {
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
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
        let path = self
            .config_file_override
            .clone()
            .unwrap_or_else(Self::config_path);
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

    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save_to_path(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let config = LauncherConfig::default();
        assert_eq!(config.max_concurrent_downloads, 64);
        assert!(config.accounts.is_empty());
        assert!(config.active_account_index.is_none());
        assert!(config.java_paths.is_empty());
        assert!(matches!(config.download_mirror, DownloadMirror::Official));
    }

    #[test]
    fn config_directories() {
        let config = LauncherConfig {
            data_dir: PathBuf::from("/tmp/test-miao"),
            ..Default::default()
        };

        assert_eq!(
            config.instances_dir(),
            PathBuf::from("/tmp/test-miao/instances")
        );
        assert_eq!(
            config.versions_dir(),
            PathBuf::from("/tmp/test-miao/versions")
        );
        assert_eq!(
            config.libraries_dir(),
            PathBuf::from("/tmp/test-miao/libraries")
        );
        assert_eq!(config.assets_dir(), PathBuf::from("/tmp/test-miao/assets"));
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let config = LauncherConfig {
            data_dir: PathBuf::from("/custom/data"),
            max_concurrent_downloads: 32,
            download_mirror: DownloadMirror::Bmclapi,
            ..Default::default()
        };

        config.save_to_path(&config_path).unwrap();
        let loaded = LauncherConfig::load_from_path(&config_path).unwrap();

        assert_eq!(loaded.data_dir, PathBuf::from("/custom/data"));
        assert_eq!(loaded.max_concurrent_downloads, 32);
        assert!(matches!(loaded.download_mirror, DownloadMirror::Bmclapi));
    }

    #[test]
    fn config_load_nonexistent_returns_default() {
        let path = PathBuf::from("/nonexistent/config.toml");
        let config = LauncherConfig::load_from_path(&path).unwrap();
        assert_eq!(config.max_concurrent_downloads, 64);
    }

    #[test]
    fn config_path_not_empty() {
        let path = LauncherConfig::config_path();
        assert!(path.to_string_lossy().contains("miao-minecraft-launcher"));
    }

    #[test]
    fn download_mirror_custom_variant() {
        let mirror = DownloadMirror::Custom("https://my-mirror.com".to_string());
        let json = serde_json::to_string(&mirror).unwrap();
        let deserialized: DownloadMirror = serde_json::from_str(&json).unwrap();
        assert!(
            matches!(deserialized, DownloadMirror::Custom(url) if url == "https://my-mirror.com")
        );
    }

    #[test]
    fn default_data_dir_not_in_tmp() {
        let config = LauncherConfig::default();
        assert!(
            !config.data_dir.starts_with("/tmp"),
            "data_dir should not be in /tmp: {:?}",
            config.data_dir
        );
    }
}
