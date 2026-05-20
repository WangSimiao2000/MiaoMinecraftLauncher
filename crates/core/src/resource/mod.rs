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

    packs.sort_by(|a, b| a.0.cmp(&b.0));
    packs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_resourcepacks_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert!(result.is_empty());
    }

    #[test]
    fn scan_resourcepacks_nonexistent() {
        let path = PathBuf::from("/nonexistent_resourcepacks_xyz");
        let result = scan_resourcepacks(&path);
        assert!(result.is_empty());
    }

    #[test]
    fn scan_resourcepacks_finds_zips() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("faithful-32x.zip"), b"").unwrap();
        fs::write(dir.path().join("vanilla-tweaks.zip"), b"").unwrap();
        fs::write(dir.path().join("readme.txt"), b"").unwrap();

        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|r| r.enabled));
    }

    #[test]
    fn scan_resourcepacks_finds_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("my-custom-pack")).unwrap();

        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "my-custom-pack");
    }

    #[test]
    fn scan_resourcepacks_name_extraction() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pack-name-v2.zip"), b"").unwrap();

        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert_eq!(result[0].name, "pack-name-v2");
        assert_eq!(result[0].file_name, "pack-name-v2.zip");
    }

    #[test]
    fn scan_shaderpacks_empty() {
        let dir = tempfile::tempdir().unwrap();
        let result = scan_shaderpacks(&dir.path().to_path_buf());
        assert!(result.is_empty());
    }

    #[test]
    fn scan_shaderpacks_finds_zips() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("BSL-v8.2.zip"), b"").unwrap();
        fs::write(dir.path().join("Complementary-4.7.1.zip"), b"").unwrap();

        let result = scan_shaderpacks(&dir.path().to_path_buf());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn scan_shaderpacks_finds_directories() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("seus-renewed")).unwrap();

        let result = scan_shaderpacks(&dir.path().to_path_buf());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "seus-renewed");
    }

    #[test]
    fn scan_packs_ignores_non_pack_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.txt"), b"").unwrap();
        fs::write(dir.path().join("notes.md"), b"").unwrap();
        fs::write(dir.path().join("image.png"), b"").unwrap();

        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert!(result.is_empty());
    }

    #[test]
    fn scan_packs_sorted_by_name() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("z-pack.zip"), b"").unwrap();
        fs::write(dir.path().join("a-pack.zip"), b"").unwrap();

        let result = scan_resourcepacks(&dir.path().to_path_buf());
        assert_eq!(result[0].name, "a-pack");
        assert_eq!(result[1].name, "z-pack");
    }
}
