use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePack {
    pub name: String,
    pub file_name: String,
    pub path: PathBuf,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderPack {
    pub name: String,
    pub file_name: String,
    pub path: PathBuf,
}

pub fn scan_resourcepacks(dir: &PathBuf) -> Vec<ResourcePack> {
    scan_packs(dir)
        .into_iter()
        .map(|(name, file_name, path)| ResourcePack {
            name,
            file_name,
            path,
            enabled: true,
        })
        .collect()
}

pub fn scan_shaderpacks(dir: &PathBuf) -> Vec<ShaderPack> {
    scan_packs(dir)
        .into_iter()
        .map(|(name, file_name, path)| ShaderPack {
            name,
            file_name,
            path,
        })
        .collect()
}

fn scan_packs(dir: &PathBuf) -> Vec<(String, String, PathBuf)> {
    let mut packs = Vec::new();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return packs;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let is_zip = file_name.ends_with(".zip");
        let is_dir = path.is_dir();

        if !is_zip && !is_dir {
            continue;
        }

        let name = file_name.trim_end_matches(".zip").to_string();
        packs.push((name, file_name, path));
    }

    packs
}
