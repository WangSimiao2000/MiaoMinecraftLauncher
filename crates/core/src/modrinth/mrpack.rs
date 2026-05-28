use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::error::Result;
use serde::{Deserialize, Serialize};

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::download::manager::DownloadManager;
use crate::http::HttpClient;
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
    http: &impl HttpClient,
    mrpack_path: &Path,
    config: &LauncherConfig,
    instance_name: Option<&str>,
) -> Result<Instance> {
    let file = std::fs::File::open(mrpack_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let index: MrpackIndex = {
        let mut index_file = archive.by_name("modrinth.index.json")?;
        let mut content = String::new();
        index_file.read_to_string(&mut content)?;
        serde_json::from_str(&content)?
    };

    let base_name = instance_name.unwrap_or(&index.name);
    let mc_version = index
        .dependencies
        .get("minecraft")
        .ok_or_else(|| {
            crate::error::MiaoError::Other(
                "No minecraft version in mrpack dependencies".to_string(),
            )
        })?
        .clone();

    let name = {
        let mut candidate = base_name.to_string();
        let mut counter = 2u32;
        while Instance::instance_dir(&config.instances_dir(), &candidate).exists() {
            candidate = format!("{}-{}", base_name, counter);
            counter += 1;
        }
        candidate
    };

    let instance_dir = Instance::instance_dir(&config.instances_dir(), &name);
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

    install_base_game(http, &mc_version, config).await?;

    let mut inst = Instance::new(&name, &mc_version);

    let loader_config = resolve_mrpack_loader(http, &index.dependencies, &mc_version, config).await;
    inst.mod_loader = loader_config;

    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    Ok(inst)
}

async fn install_base_game(
    http: &impl HttpClient,
    mc_version: &str,
    config: &LauncherConfig,
) -> Result<()> {
    use crate::download::mirror::transform_url;
    use crate::version::{assets, install, manifest};

    let all_versions = manifest::fetch_version_manifest(http, &config.download_mirror).await?;
    let version_info = all_versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| {
            crate::error::MiaoError::Other(format!(
                "Minecraft version '{}' not found in manifest",
                mc_version
            ))
        })?;

    let version_url = transform_url(&version_info.url, &config.download_mirror);
    let meta = http.get_json(&version_url).await?;
    install::save_version_meta(&meta, config)?;

    let mut all_tasks = install::all_download_tasks(&meta, config, &config.download_mirror);
    let native_tasks = install::collect_native_downloads(&meta, config, &config.download_mirror);
    all_tasks.extend(native_tasks);

    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(all_tasks).await?;
    install::extract_natives(&meta, config)?;

    let asset_index_task =
        install::collect_asset_index_download(&meta, config, &config.download_mirror);
    let asset_index_path = asset_index_task.dest.clone();
    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(vec![asset_index_task]).await?;

    if asset_index_path.exists() {
        let asset_index = assets::fetch_asset_index(&asset_index_path).await?;
        let asset_tasks =
            assets::collect_asset_downloads(&asset_index, config, &config.download_mirror);
        let dm = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm.download_all(asset_tasks).await?;
    }

    Ok(())
}

async fn resolve_mrpack_loader(
    http: &impl HttpClient,
    dependencies: &std::collections::HashMap<String, String>,
    mc_version: &str,
    config: &LauncherConfig,
) -> Option<crate::instance::ModLoaderConfig> {
    use crate::modloader::{self, ModLoaderType};

    let (loader_type, loader_version) = if let Some(ver) = dependencies.get("fabric-loader") {
        (ModLoaderType::Fabric, ver.clone())
    } else if let Some(ver) = dependencies.get("quilt-loader") {
        (ModLoaderType::Quilt, ver.clone())
    } else if let Some(ver) = dependencies.get("neoforge") {
        (ModLoaderType::NeoForge, ver.clone())
    } else if let Some(ver) = dependencies.get("forge") {
        (ModLoaderType::Forge, ver.clone())
    } else {
        return None;
    };

    match modloader::install_loader(http, &loader_type, mc_version, &loader_version, config).await {
        Ok(loader_config) => Some(loader_config),
        Err(_) => Some(crate::instance::ModLoaderConfig {
            loader_type,
            version: loader_version,
            main_class: None,
            extra_libraries: Vec::new(),
        }),
    }
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
    use std::io::Read;

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

    #[test]
    fn mrpack_index_with_env_field() {
        let json = r#"{
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0.0",
            "name": "Env Pack",
            "files": [{
                "path": "mods/client-only.jar",
                "hashes": {"sha1": "abc", "sha512": "def"},
                "env": {"client": "required", "server": "unsupported"},
                "downloads": ["https://example.com/mod.jar"],
                "fileSize": 100
            }],
            "dependencies": {"minecraft": "1.20.4"}
        }"#;

        let index: MrpackIndex = serde_json::from_str(json).unwrap();
        let env = index.files[0].env.as_ref().unwrap();
        assert_eq!(env.client, "required");
        assert_eq!(env.server, "unsupported");
    }

    fn setup_instance_for_export(dir: &Path) -> Instance {
        let mods_dir = dir.join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();
        std::fs::write(mods_dir.join("sodium.jar"), b"sodium mod data").unwrap();
        std::fs::write(mods_dir.join("iris.jar"), b"iris mod data").unwrap();
        std::fs::write(mods_dir.join("disabled.jar.disabled"), b"disabled").unwrap();

        let mut inst = Instance::new("test-instance", "1.20.4");
        inst.mod_loader = Some(crate::instance::ModLoaderConfig {
            loader_type: crate::modloader::ModLoaderType::Fabric,
            version: "0.15.6".to_string(),
            main_class: None,
            extra_libraries: Vec::new(),
        });
        inst.save_to(dir).unwrap();
        inst
    }

    #[test]
    fn export_mrpack_creates_valid_zip() {
        let tmp = tempfile::tempdir().unwrap();
        let instance_dir = tmp.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let inst = setup_instance_for_export(&instance_dir);

        let output_dir = tmp.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let result = export_mrpack(&instance_dir, &inst, &output_dir).unwrap();
        assert!(result.exists());
        assert_eq!(
            result.file_name().unwrap().to_str().unwrap(),
            "test-instance.mrpack"
        );

        let file = std::fs::File::open(&result).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let mut index_content = String::new();
        archive
            .by_name("modrinth.index.json")
            .unwrap()
            .read_to_string(&mut index_content)
            .unwrap();

        let index: MrpackIndex = serde_json::from_str(&index_content).unwrap();
        assert_eq!(index.name, "test-instance");
        assert_eq!(index.dependencies["minecraft"], "1.20.4");
        assert_eq!(index.dependencies["fabric-loader"], "0.15.6");
        assert_eq!(index.files.len(), 2);
    }

    #[test]
    fn export_mrpack_computes_hashes() {
        let tmp = tempfile::tempdir().unwrap();
        let instance_dir = tmp.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let inst = setup_instance_for_export(&instance_dir);

        let output = tmp.path().join("out.mrpack");
        export_mrpack(&instance_dir, &inst, &output).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let mut index_content = String::new();
        archive
            .by_name("modrinth.index.json")
            .unwrap()
            .read_to_string(&mut index_content)
            .unwrap();

        let index: MrpackIndex = serde_json::from_str(&index_content).unwrap();
        for f in &index.files {
            assert!(!f.hashes.sha1.is_empty());
            assert!(!f.hashes.sha512.is_empty());
            assert!(f.file_size > 0);
        }
    }

    #[test]
    fn export_mrpack_excludes_disabled_mods() {
        let tmp = tempfile::tempdir().unwrap();
        let instance_dir = tmp.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let inst = setup_instance_for_export(&instance_dir);

        let output = tmp.path().join("test.mrpack");
        export_mrpack(&instance_dir, &inst, &output).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();

        let mut index_content = String::new();
        archive
            .by_name("modrinth.index.json")
            .unwrap()
            .read_to_string(&mut index_content)
            .unwrap();

        let index: MrpackIndex = serde_json::from_str(&index_content).unwrap();
        for f in &index.files {
            assert!(!f.path.contains("disabled"));
        }
    }

    #[test]
    fn export_mrpack_includes_overrides() {
        let tmp = tempfile::tempdir().unwrap();
        let instance_dir = tmp.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let inst = setup_instance_for_export(&instance_dir);

        std::fs::write(instance_dir.join("options.txt"), b"fov:90").unwrap();
        let config_dir = instance_dir.join("config");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(config_dir.join("mod.toml"), b"setting=true").unwrap();

        let output = tmp.path().join("override.mrpack");
        export_mrpack(&instance_dir, &inst, &output).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();

        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert!(names.contains(&"overrides/options.txt".to_string()));
        assert!(names.contains(&"overrides/config/mod.toml".to_string()));
    }

    #[test]
    fn export_mrpack_no_loader_only_minecraft_dep() {
        let tmp = tempfile::tempdir().unwrap();
        let instance_dir = tmp.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();

        let mods_dir = instance_dir.join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();

        let inst = Instance::new("vanilla", "1.20.4");
        inst.save_to(&instance_dir).unwrap();

        let output = tmp.path().join("vanilla.mrpack");
        export_mrpack(&instance_dir, &inst, &output).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut content = String::new();
        archive
            .by_name("modrinth.index.json")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();

        let index: MrpackIndex = serde_json::from_str(&content).unwrap();
        assert_eq!(index.dependencies.len(), 1);
        assert_eq!(index.dependencies["minecraft"], "1.20.4");
    }

    #[test]
    fn export_mrpack_all_loader_types_in_deps() {
        let tmp = tempfile::tempdir().unwrap();

        let loaders = [
            (crate::modloader::ModLoaderType::Fabric, "fabric-loader"),
            (crate::modloader::ModLoaderType::Quilt, "quilt-loader"),
            (crate::modloader::ModLoaderType::NeoForge, "neoforge"),
            (crate::modloader::ModLoaderType::Forge, "forge"),
        ];

        for (loader_type, expected_key) in &loaders {
            let instance_dir = tmp.path().join(format!("inst-{}", expected_key));
            std::fs::create_dir_all(&instance_dir).unwrap();
            let mods_dir = instance_dir.join("mods");
            std::fs::create_dir_all(&mods_dir).unwrap();

            let mut inst = Instance::new("test", "1.20.4");
            inst.mod_loader = Some(crate::instance::ModLoaderConfig {
                loader_type: loader_type.clone(),
                version: "1.0.0".to_string(),
                main_class: None,
                extra_libraries: Vec::new(),
            });
            inst.save_to(&instance_dir).unwrap();

            let output = tmp.path().join(format!("{}.mrpack", expected_key));
            export_mrpack(&instance_dir, &inst, &output).unwrap();

            let file = std::fs::File::open(&output).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();
            let mut content = String::new();
            archive
                .by_name("modrinth.index.json")
                .unwrap()
                .read_to_string(&mut content)
                .unwrap();

            let index: MrpackIndex = serde_json::from_str(&content).unwrap();
            assert!(
                index.dependencies.contains_key(*expected_key),
                "Expected key '{}' in dependencies",
                expected_key
            );
            assert_eq!(index.dependencies[*expected_key], "1.0.0");
        }
    }
}
