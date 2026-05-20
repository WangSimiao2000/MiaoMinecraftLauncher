use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::modloader::ModLoaderType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModLoaderConfig {
    pub loader_type: ModLoaderType,
    pub version: String,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub extra_libraries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub fn new(name: &str, minecraft_version: &str) -> Self {
        Self {
            name: name.to_string(),
            minecraft_version: minecraft_version.to_string(),
            ..Default::default()
        }
    }

    pub fn with_mod_loader(mut self, loader_type: ModLoaderType, version: &str) -> Self {
        self.mod_loader = Some(ModLoaderConfig {
            loader_type,
            version: version.to_string(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
        self
    }

    pub fn with_memory(mut self, min_mb: u32, max_mb: u32) -> Self {
        self.memory_min_mb = min_mb;
        self.memory_max_mb = max_mb;
        self
    }

    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = Some(Resolution { width, height });
        self
    }

    pub fn instance_dir(base: &Path, name: &str) -> PathBuf {
        base.join(name)
    }

    pub fn mods_dir(instance_dir: &Path) -> PathBuf {
        instance_dir.join("mods")
    }

    pub fn resourcepacks_dir(instance_dir: &Path) -> PathBuf {
        instance_dir.join("resourcepacks")
    }

    pub fn shaderpacks_dir(instance_dir: &Path) -> PathBuf {
        instance_dir.join("shaderpacks")
    }

    pub fn saves_dir(instance_dir: &Path) -> PathBuf {
        instance_dir.join("saves")
    }

    pub fn config_file(instance_dir: &Path) -> PathBuf {
        instance_dir.join("instance.toml")
    }

    pub fn save_to(&self, instance_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(instance_dir)?;
        let config_path = Self::config_file(instance_dir);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    pub fn load_from(instance_dir: &Path) -> Result<Self> {
        let config_path = Self::config_file(instance_dir);
        let content = std::fs::read_to_string(&config_path)?;
        let instance: Self = toml::from_str(&content)?;
        Ok(instance)
    }

    pub fn create_directories(instance_dir: &Path) -> Result<()> {
        std::fs::create_dir_all(Self::mods_dir(instance_dir))?;
        std::fs::create_dir_all(Self::resourcepacks_dir(instance_dir))?;
        std::fs::create_dir_all(Self::shaderpacks_dir(instance_dir))?;
        std::fs::create_dir_all(Self::saves_dir(instance_dir))?;
        Ok(())
    }
}

pub fn list_instances(instances_base: &Path) -> Result<Vec<Instance>> {
    let mut instances = Vec::new();

    if !instances_base.exists() {
        return Ok(instances);
    }

    for entry in std::fs::read_dir(instances_base)?.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let config_file = Instance::config_file(&path);
        if config_file.exists()
            && let Ok(instance) = Instance::load_from(&path)
        {
            instances.push(instance);
        }
    }

    instances.sort_by_key(|i| std::cmp::Reverse(i.last_played));
    Ok(instances)
}

pub fn delete_instance(instances_base: &Path, name: &str) -> Result<()> {
    let dir = Instance::instance_dir(instances_base, name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

pub fn open_folder(path: &Path) -> Result<()> {
    if path.exists() {
        open::that(path)?;
    } else {
        anyhow::bail!("Directory does not exist: {}", path.display());
    }
    Ok(())
}

pub fn list_saves(instance_dir: &Path) -> Vec<SaveWorld> {
    let saves_dir = Instance::saves_dir(instance_dir);
    let mut worlds = Vec::new();

    let Ok(entries) = std::fs::read_dir(&saves_dir) else {
        return worlds;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let level_dat = path.join("level.dat");
        if !level_dat.exists() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        worlds.push(SaveWorld { name, path });
    }

    worlds.sort_by(|a, b| a.name.cmp(&b.name));
    worlds
}

#[derive(Debug, Clone)]
pub struct SaveWorld {
    pub name: String,
    pub path: PathBuf,
}

impl SaveWorld {
    pub fn delete(&self) -> Result<()> {
        std::fs::remove_dir_all(&self.path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_default_values() {
        let inst = Instance::default();
        assert_eq!(inst.memory_max_mb, 4096);
        assert_eq!(inst.memory_min_mb, 512);
        assert!(inst.mod_loader.is_none());
        assert!(inst.java_path.is_none());
        assert!(inst.resolution.is_none());
        assert!(inst.last_played.is_none());
    }

    #[test]
    fn instance_new_sets_name_and_version() {
        let inst = Instance::new("test-instance", "1.20.4");
        assert_eq!(inst.name, "test-instance");
        assert_eq!(inst.minecraft_version, "1.20.4");
    }

    #[test]
    fn instance_builder_chain() {
        let inst = Instance::new("modded", "1.20.4")
            .with_mod_loader(ModLoaderType::Fabric, "0.15.6")
            .with_memory(1024, 8192)
            .with_resolution(1920, 1080);

        assert_eq!(inst.memory_min_mb, 1024);
        assert_eq!(inst.memory_max_mb, 8192);
        assert_eq!(
            inst.mod_loader,
            Some(ModLoaderConfig {
                loader_type: ModLoaderType::Fabric,
                version: "0.15.6".to_string(),
                main_class: None,
                extra_libraries: Vec::new(),
            })
        );
        assert_eq!(
            inst.resolution,
            Some(Resolution {
                width: 1920,
                height: 1080
            })
        );
    }

    #[test]
    fn instance_dir_paths() {
        let base = PathBuf::from("/home/user/.local/share/miao/instances");
        let dir = Instance::instance_dir(&base, "my-world");

        assert_eq!(
            dir,
            PathBuf::from("/home/user/.local/share/miao/instances/my-world")
        );
        assert!(Instance::mods_dir(&dir).ends_with("mods"));
        assert!(Instance::resourcepacks_dir(&dir).ends_with("resourcepacks"));
        assert!(Instance::shaderpacks_dir(&dir).ends_with("shaderpacks"));
        assert!(Instance::saves_dir(&dir).ends_with("saves"));
        assert!(Instance::config_file(&dir).ends_with("instance.toml"));
    }

    #[test]
    fn instance_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let instance_dir = dir.path().join("test-instance");

        let inst = Instance::new("test-instance", "1.20.4")
            .with_mod_loader(ModLoaderType::Forge, "47.2.0")
            .with_memory(2048, 6144)
            .with_resolution(1280, 720);

        inst.save_to(&instance_dir.to_path_buf()).unwrap();

        let loaded = Instance::load_from(&instance_dir.to_path_buf()).unwrap();
        assert_eq!(loaded.name, "test-instance");
        assert_eq!(loaded.minecraft_version, "1.20.4");
        assert_eq!(loaded.memory_min_mb, 2048);
        assert_eq!(loaded.memory_max_mb, 6144);
        assert_eq!(loaded.mod_loader.unwrap().loader_type, ModLoaderType::Forge);
        assert_eq!(loaded.resolution.unwrap().width, 1280);
    }

    #[test]
    fn instance_create_directories() {
        let dir = tempfile::tempdir().unwrap();
        let instance_dir = dir.path().join("new-instance");

        Instance::create_directories(&instance_dir.to_path_buf()).unwrap();

        assert!(instance_dir.join("mods").exists());
        assert!(instance_dir.join("resourcepacks").exists());
        assert!(instance_dir.join("shaderpacks").exists());
        assert!(instance_dir.join("saves").exists());
    }

    #[test]
    fn list_instances_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let result = list_instances(dir.path()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_instances_nonexistent_dir() {
        let path = PathBuf::from("/nonexistent_instances_dir_xyz");
        let result = list_instances(&path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_instances_finds_saved() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();

        Instance::new("world-1", "1.20.4")
            .save_to(&base.join("world-1"))
            .unwrap();
        Instance::new("world-2", "1.19.4")
            .save_to(&base.join("world-2"))
            .unwrap();

        let result = list_instances(&base).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn delete_instance_removes_dir() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let instance_dir = base.join("to-delete");

        Instance::new("to-delete", "1.20.4")
            .save_to(&instance_dir)
            .unwrap();
        Instance::create_directories(&instance_dir).unwrap();
        assert!(instance_dir.exists());

        delete_instance(&base, "to-delete").unwrap();
        assert!(!instance_dir.exists());
    }

    #[test]
    fn delete_instance_nonexistent_ok() {
        let dir = tempfile::tempdir().unwrap();
        assert!(delete_instance(dir.path(), "nope").is_ok());
    }

    #[test]
    fn instance_load_nonexistent_errors() {
        let path = PathBuf::from("/nonexistent/instance");
        assert!(Instance::load_from(&path).is_err());
    }
}
