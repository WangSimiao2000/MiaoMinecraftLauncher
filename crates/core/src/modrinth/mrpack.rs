use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::download::manager::DownloadManager;
use crate::instance::Instance;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrpackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<MrpackFile>,
    pub dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrpackFile {
    pub path: String,
    pub hashes: MrpackHashes,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<MrpackEnv>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MrpackEnv {
    pub client: String,
    pub server: String,
}

pub async fn import_mrpack(
    mrpack_path: &Path,
    config: &LauncherConfig,
    instance_name: Option<&str>,
) -> Result<Instance> {
    let file = std::fs::File::open(mrpack_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let index: MrpackIndex = {
        let mut index_file = archive
            .by_name("modrinth.index.json")
            .context("modrinth.index.json not found in mrpack")?;
        let mut content = String::new();
        index_file.read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };

    let name = instance_name.unwrap_or(&index.name);
    let mc_version = index
        .dependencies
        .get("minecraft")
        .ok_or_else(|| anyhow::anyhow!("No minecraft version in mrpack dependencies"))?
        .clone();

    let instance_dir = Instance::instance_dir(&config.instances_dir(), name);
    std::fs::create_dir_all(&instance_dir)?;

    let mut tasks: Vec<DownloadTask> = Vec::new();
    for f in &index.files {
        let dest = instance_dir.join(&f.path);
        if let Some(url) = f.downloads.first() {
            tasks.push(DownloadTask {
                url: url.clone(),
                dest,
                sha1: Some(f.hashes.sha1.clone()),
                size: Some(f.file_size),
            });
        }
    }

    if !tasks.is_empty() {
        let dm = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm.download_all(tasks).await?;
    }

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();

        let strip_prefix = if entry_name.starts_with("overrides/") {
            Some("overrides/")
        } else if entry_name.starts_with("client-overrides/") {
            Some("client-overrides/")
        } else {
            None
        };

        if let Some(prefix) = strip_prefix {
            let relative = entry_name.strip_prefix(prefix).unwrap_or(&entry_name);
            if relative.is_empty() || relative.ends_with('/') {
                let dir = instance_dir.join(relative);
                std::fs::create_dir_all(dir)?;
            } else {
                let dest = instance_dir.join(relative);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&dest, &buf)?;
            }
        }
    }

    let mut inst = Instance::new(name, &mc_version);

    if let Some(fabric_ver) = index.dependencies.get("fabric-loader") {
        inst.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: fabric_ver.clone(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
    } else if let Some(quilt_ver) = index.dependencies.get("quilt-loader") {
        inst.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Quilt,
            version: quilt_ver.clone(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
    } else if let Some(neoforge_ver) = index.dependencies.get("neoforge") {
        inst.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::NeoForge,
            version: neoforge_ver.clone(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
    } else if let Some(forge_ver) = index.dependencies.get("forge") {
        inst.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Forge,
            version: forge_ver.clone(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
    }

    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    Ok(inst)
}

pub fn export_mrpack(instance_dir: &Path, inst: &Instance, output_path: &Path) -> Result<PathBuf> {
    let mods_dir = Instance::mods_dir(instance_dir);
    let mods = crate::modmanager::scan_mods_dir(&mods_dir);

    let mut files: Vec<MrpackFile> = Vec::new();
    for m in &mods {
        if !m.enabled {
            continue;
        }
        let data = std::fs::read(&m.path)?;
        let sha1 = {
            use sha1::Digest;
            let hash = sha1::Sha1::digest(&data);
            format!("{:x}", hash)
        };
        let sha512 = {
            use sha2::Digest;
            let hash = sha2::Sha512::digest(&data);
            format!("{:x}", hash)
        };

        files.push(MrpackFile {
            path: format!("mods/{}", m.file_name),
            hashes: MrpackHashes { sha1, sha512 },
            env: None,
            downloads: Vec::new(),
            file_size: data.len() as u64,
        });
    }

    let mut dependencies = std::collections::HashMap::new();
    dependencies.insert("minecraft".to_string(), inst.minecraft_version.clone());
    if let Some(ref loader) = inst.mod_loader {
        let key = match loader.loader_type {
            crate::modloader::ModLoaderType::Fabric => "fabric-loader",
            crate::modloader::ModLoaderType::Quilt => "quilt-loader",
            crate::modloader::ModLoaderType::NeoForge => "neoforge",
            crate::modloader::ModLoaderType::Forge => "forge",
        };
        dependencies.insert(key.to_string(), loader.version.clone());
    }

    let index = MrpackIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: format!("{}-export", inst.minecraft_version),
        name: inst.name.clone(),
        summary: None,
        files,
        dependencies,
    };

    let output = if output_path.is_dir() {
        output_path.join(format!("{}.mrpack", inst.name))
    } else {
        output_path.to_path_buf()
    };

    let zip_file = std::fs::File::create(&output)?;
    let mut zip = zip::ZipWriter::new(zip_file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("modrinth.index.json", options)?;
    let index_json = serde_json::to_string_pretty(&index)?;
    zip.write_all(index_json.as_bytes())?;

    let override_dirs = ["config", "options.txt"];
    for name in &override_dirs {
        let path = instance_dir.join(name);
        if path.exists() {
            if path.is_file() {
                let data = std::fs::read(&path)?;
                zip.start_file(format!("overrides/{}", name), options)?;
                zip.write_all(&data)?;
            } else if path.is_dir() {
                add_directory_to_zip(&mut zip, &path, &format!("overrides/{}", name), options)?;
            }
        }
    }

    for m in &mods {
        if !m.enabled {
            continue;
        }
        let data = std::fs::read(&m.path)?;
        zip.start_file(format!("overrides/mods/{}", m.file_name), options)?;
        zip.write_all(&data)?;
    }

    zip.finish()?;
    Ok(output)
}

fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    dir: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let zip_path = format!("{}/{}", prefix, name);

        if path.is_file() {
            let data = std::fs::read(&path)?;
            zip.start_file(&zip_path, options)?;
            zip.write_all(&data)?;
        } else if path.is_dir() {
            add_directory_to_zip(zip, &path, &zip_path, options)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mrpack_index_deserializes() {
        let json = r#"{
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0.0",
            "name": "Test Pack",
            "summary": "A test modpack",
            "files": [{
                "path": "mods/sodium.jar",
                "hashes": {"sha1": "abc", "sha512": "def"},
                "downloads": ["https://cdn.modrinth.com/test.jar"],
                "fileSize": 949085
            }],
            "dependencies": {
                "minecraft": "1.20.4",
                "fabric-loader": "0.15.6"
            }
        }"#;

        let index: MrpackIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.name, "Test Pack");
        assert_eq!(index.files.len(), 1);
        assert_eq!(index.dependencies["minecraft"], "1.20.4");
        assert_eq!(index.dependencies["fabric-loader"], "0.15.6");
    }

    #[test]
    fn mrpack_index_serializes() {
        let index = MrpackIndex {
            format_version: 1,
            game: "minecraft".to_string(),
            version_id: "1.0.0".to_string(),
            name: "My Pack".to_string(),
            summary: None,
            files: Vec::new(),
            dependencies: [("minecraft".to_string(), "1.20.4".to_string())]
                .into_iter()
                .collect(),
        };

        let json = serde_json::to_string(&index).unwrap();
        assert!(json.contains("\"formatVersion\":1"));
        assert!(json.contains("\"name\":\"My Pack\""));
    }
}
