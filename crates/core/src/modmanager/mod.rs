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

    mods
}
