use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::modloader::ModLoaderType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub file_name: String,
    pub version: Option<String>,
    pub loader: ModLoaderType,
    pub enabled: bool,
    pub path: PathBuf,
}

impl ModInfo {
    pub fn toggle(&mut self) -> std::io::Result<()> {
        let new_path = if self.enabled {
            self.path.with_extension("jar.disabled")
        } else {
            let name = self.path.file_stem().unwrap_or_default().to_string_lossy();
            let name = name.strip_suffix(".jar").unwrap_or(&name);
            self.path.with_file_name(format!("{}.jar", name))
        };

        std::fs::rename(&self.path, &new_path)?;
        self.path = new_path;
        self.enabled = !self.enabled;
        Ok(())
    }
}

pub fn scan_mods_dir(mods_dir: &PathBuf) -> Vec<ModInfo> {
    let mut mods = Vec::new();

    let Ok(entries) = std::fs::read_dir(mods_dir) else {
        return mods;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_jar = file_name.ends_with(".jar");
        let is_disabled = file_name.ends_with(".jar.disabled");

        if !is_jar && !is_disabled {
            continue;
        }

        mods.push(ModInfo {
            name: file_name
                .trim_end_matches(".disabled")
                .trim_end_matches(".jar")
                .to_string(),
            file_name: file_name.clone(),
            version: None,
            loader: ModLoaderType::Fabric,
            enabled: is_jar,
            path,
        });
    }

    mods.sort_by(|a, b| a.name.cmp(&b.name));
    mods
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_mods_dir_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_mods_dir(&dir.path().to_path_buf());
        assert!(result.is_empty());
    }

    #[test]
    fn scan_mods_dir_nonexistent() {
        let path = PathBuf::from("/nonexistent_mods_dir_xyz");
        let result = scan_mods_dir(&path);
        assert!(result.is_empty());
    }

    #[test]
    fn scan_mods_dir_finds_jars() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("sodium-0.5.8.jar"), b"").unwrap();
        fs::write(dir.path().join("iris-1.6.11.jar"), b"").unwrap();
        fs::write(dir.path().join("readme.txt"), b"").unwrap();

        let result = scan_mods_dir(&dir.path().to_path_buf());
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|m| m.enabled));
    }

    #[test]
    fn scan_mods_dir_finds_disabled() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("sodium-0.5.8.jar.disabled"), b"").unwrap();

        let result = scan_mods_dir(&dir.path().to_path_buf());
        assert_eq!(result.len(), 1);
        assert!(!result[0].enabled);
        assert_eq!(result[0].name, "sodium-0.5.8");
    }

    #[test]
    fn scan_mods_dir_ignores_non_jar() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.json"), b"{}").unwrap();
        fs::write(dir.path().join("notes.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let result = scan_mods_dir(&dir.path().to_path_buf());
        assert!(result.is_empty());
    }

    #[test]
    fn mod_toggle_disable() {
        let dir = tempfile::tempdir().unwrap();
        let jar_path = dir.path().join("test-mod.jar");
        fs::write(&jar_path, b"fake jar").unwrap();

        let mut mod_info = ModInfo {
            name: "test-mod".to_string(),
            file_name: "test-mod.jar".to_string(),
            version: None,
            loader: ModLoaderType::Fabric,
            enabled: true,
            path: jar_path.clone(),
        };

        mod_info.toggle().unwrap();

        assert!(!mod_info.enabled);
        assert!(mod_info.path.to_string_lossy().ends_with(".jar.disabled"));
        assert!(mod_info.path.exists());
        assert!(!jar_path.exists());
    }

    #[test]
    fn mod_toggle_enable() {
        let dir = tempfile::tempdir().unwrap();
        let disabled_path = dir.path().join("test-mod.jar.disabled");
        fs::write(&disabled_path, b"fake jar").unwrap();

        let mut mod_info = ModInfo {
            name: "test-mod".to_string(),
            file_name: "test-mod.jar.disabled".to_string(),
            version: None,
            loader: ModLoaderType::Fabric,
            enabled: false,
            path: disabled_path.clone(),
        };

        mod_info.toggle().unwrap();

        assert!(mod_info.enabled);
        assert!(mod_info.path.to_string_lossy().ends_with(".jar"));
        assert!(!mod_info.path.to_string_lossy().ends_with(".disabled"));
        assert!(mod_info.path.exists());
        assert!(!disabled_path.exists());
    }

    #[test]
    fn mod_toggle_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let jar_path = dir.path().join("roundtrip-mod.jar");
        fs::write(&jar_path, b"content").unwrap();

        let mut mod_info = ModInfo {
            name: "roundtrip-mod".to_string(),
            file_name: "roundtrip-mod.jar".to_string(),
            version: None,
            loader: ModLoaderType::Forge,
            enabled: true,
            path: jar_path,
        };

        mod_info.toggle().unwrap();
        assert!(!mod_info.enabled);

        mod_info.toggle().unwrap();
        assert!(mod_info.enabled);
        assert!(mod_info.path.exists());
    }

    #[test]
    fn scan_mods_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("z-mod.jar"), b"").unwrap();
        fs::write(dir.path().join("a-mod.jar"), b"").unwrap();
        fs::write(dir.path().join("m-mod.jar"), b"").unwrap();

        let result = scan_mods_dir(&dir.path().to_path_buf());
        assert_eq!(result[0].name, "a-mod");
        assert_eq!(result[1].name, "m-mod");
        assert_eq!(result[2].name, "z-mod");
    }
}
