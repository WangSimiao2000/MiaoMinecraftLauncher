use std::path::PathBuf;

use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::auth::AuthMethod;

const LOCALE_EXAMPLE_TEMPLATE: &str = r#"{
  "_name": "Language Name (native script)",
  "ready": "Ready",
  "settings": "Settings",
  "instances": "Instances",
  "new": "+ New",
  "import": "Import",
  "launch": "▶ Launch",
  "search_mods": "Search Mods",
  "welcome": "Welcome to MMCL",
  "welcome_hint": "Create or select an instance to get started."
}
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub data_dir: PathBuf,
    #[serde(default)]
    pub game_folders: Vec<PathBuf>,
    pub java_paths: Vec<PathBuf>,
    pub download_mirror: DownloadMirror,
    #[serde(default)]
    pub java_source: JavaSource,
    pub max_concurrent_downloads: usize,
    pub accounts: Vec<AuthMethod>,
    pub active_account_index: Option<usize>,
    #[serde(default)]
    pub curseforge_api_key: Option<String>,
    #[serde(default)]
    pub theme: ThemePreset,
    #[serde(default)]
    pub custom_theme_name: Option<String>,
    #[serde(
        default = "default_language",
        deserialize_with = "deserialize_language"
    )]
    pub language: String,
    #[serde(default)]
    pub setup_complete: bool,
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
    Sakura,
    Light,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 6] = [
        ThemePreset::Dark,
        ThemePreset::Ocean,
        ThemePreset::Forest,
        ThemePreset::Warm,
        ThemePreset::Sakura,
        ThemePreset::Light,
    ];

    pub fn i18n_key(&self) -> &'static str {
        match self {
            ThemePreset::Dark => "theme_dark",
            ThemePreset::Ocean => "theme_ocean",
            ThemePreset::Forest => "theme_forest",
            ThemePreset::Warm => "theme_warm",
            ThemePreset::Sakura => "theme_sakura",
            ThemePreset::Light => "theme_light",
        }
    }
}

/// User language preference. Lives in core only as a serializable enum so the
pub fn default_language() -> String {
    "en".to_string()
}

fn deserialize_language<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;
    struct LangVisitor;
    impl<'de> de::Visitor<'de> for LangVisitor {
        type Value = String;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a language string like \"en\" or legacy enum like \"English\"")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<String, E> {
            Ok(match v {
                "English" => "en".to_string(),
                "Chinese" => "zh".to_string(),
                other => other.to_string(),
            })
        }
    }
    deserializer.deserialize_str(LangVisitor)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum DownloadMirror {
    #[default]
    Official,
    Bmclapi,
    Custom(String),
}

/// Where to fetch Java runtimes from when the user requests a Java install.
///
/// New sources should be added here, in [`crate::java::install::JavaSource::all`],
/// and given a planner in [`crate::java::install::plan`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum JavaSource {
    #[default]
    Mojang,
    Bmclapi,
    Adoptium,
    Microsoft,
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
            java_source: JavaSource::default(),
            max_concurrent_downloads: 64,
            accounts: Vec::new(),
            active_account_index: None,
            curseforge_api_key: None,
            theme: ThemePreset::default(),
            custom_theme_name: None,
            language: default_language(),
            setup_complete: false,
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
            let mut config: Self = toml::from_str(&content)?;
            config.setup_complete = true;
            Ok(config)
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

    pub fn themes_dir(&self) -> PathBuf {
        self.data_dir.join("themes")
    }

    pub fn locales_dir(&self) -> PathBuf {
        self.data_dir.join("locales")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// HTTP cache root for the modpack-source ecosystem (Phase 1: manifest +
    /// pack.json). Symmetric with `themes_dir()` / `locales_dir()`. Lazily
    /// populated by `core::modpack_source::cache::Cache` on first write.
    pub fn modpack_cache_dir(&self) -> PathBuf {
        self.data_dir.join("modpack-cache")
    }

    pub fn ensure_data_dirs(&self) {
        let dirs = [self.instances_dir(), self.themes_dir(), self.locales_dir()];
        for dir in &dirs {
            let _ = std::fs::create_dir_all(dir);
        }
        let theme_example = self.themes_dir().join("_example.toml");
        if !theme_example.exists() {
            let content = crate::custom_theme::generate_example_theme();
            let _ = std::fs::write(&theme_example, content);
        }
        let locale_example = self.locales_dir().join("_example.json");
        if !locale_example.exists() {
            let _ = std::fs::write(&locale_example, LOCALE_EXAMPLE_TEMPLATE);
        }
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
