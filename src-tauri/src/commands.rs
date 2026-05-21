use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::modmanager;
use miao_core::version::VersionInfo;
use serde::Serialize;

fn load_config() -> LauncherConfig {
    LauncherConfig::load().unwrap_or_default()
}

#[derive(Serialize)]
pub struct InstanceInfo {
    pub name: String,
    pub minecraft_version: String,
    pub loader_type: Option<String>,
    pub loader_version: Option<String>,
}

impl From<&Instance> for InstanceInfo {
    fn from(inst: &Instance) -> Self {
        Self {
            name: inst.name.clone(),
            minecraft_version: inst.minecraft_version.clone(),
            loader_type: inst
                .mod_loader
                .as_ref()
                .map(|l| l.loader_type.as_str().to_string()),
            loader_version: inst.mod_loader.as_ref().map(|l| l.version.clone()),
        }
    }
}

#[derive(Serialize)]
pub struct ModItem {
    pub name: String,
    pub filename: String,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct JavaInfo {
    pub major_version: u32,
    pub version: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct ConfigInfo {
    pub data_dir: String,
    pub max_concurrent_downloads: usize,
    pub accounts: Vec<String>,
    pub active_account: Option<usize>,
}

#[tauri::command]
pub fn list_instances() -> Vec<InstanceInfo> {
    let config = load_config();
    let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();
    instances.iter().map(InstanceInfo::from).collect()
}

#[tauri::command]
pub async fn get_versions() -> Result<Vec<VersionInfo>, String> {
    let config = load_config();
    let http = reqwest::Client::new();
    miao_core::version::manifest::fetch_version_manifest(&http, &config.download_mirror)
        .await
        .map(|v| v.into_iter().filter(|v| v.is_release()).take(50).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_instance(
    version: String,
    name: Option<String>,
    loader_type: Option<String>,
    loader_version: Option<String>,
) -> Result<String, String> {
    let config = load_config();
    let http = reqwest::Client::new();

    let versions =
        miao_core::version::manifest::fetch_version_manifest(&http, &config.download_mirror)
            .await
            .map_err(|e| e.to_string())?;

    let ver = versions
        .iter()
        .find(|v| v.id == version)
        .ok_or_else(|| format!("Version '{}' not found", version))?;

    let instance_name = name.unwrap_or_else(|| version.clone());

    let meta =
        miao_core::version::install::fetch_version_meta(&http, &ver.url, &config.download_mirror)
            .await
            .map_err(|e| e.to_string())?;
    miao_core::version::install::save_version_meta(&meta, &config).map_err(|e| e.to_string())?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, &config, &config.download_mirror);
    let dm = miao_core::download::manager::DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(tasks).await.map_err(|e| e.to_string())?;

    let asset_index_path = config
        .assets_dir()
        .join("indexes")
        .join(format!("{}.json", &meta.asset_index.id));
    if asset_index_path.exists() {
        let asset_index = miao_core::version::assets::fetch_asset_index(&asset_index_path)
            .await
            .map_err(|e| e.to_string())?;
        let asset_tasks = miao_core::version::assets::collect_asset_downloads(
            &asset_index,
            &config,
            &config.download_mirror,
        );
        let dm2 = miao_core::download::manager::DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks)
            .await
            .map_err(|e| e.to_string())?;
    }

    let mut inst = Instance::new(&instance_name, &version);

    if let (Some(lt_str), Some(lv)) = (loader_type, loader_version) {
        let lt = miao_core::modloader::ModLoaderType::ALL
            .iter()
            .find(|t| t.as_str() == lt_str)
            .cloned()
            .ok_or_else(|| format!("Unknown loader: {}", lt_str))?;
        let loader_config =
            miao_core::modloader::install_loader(&http, &lt, &version, &lv, &config)
                .await
                .map_err(|e| e.to_string())?;
        inst.mod_loader = Some(loader_config);
    }

    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    inst.save_to(&instance_dir).map_err(|e| e.to_string())?;
    Instance::create_directories(&instance_dir).map_err(|e| e.to_string())?;

    Ok(instance_name)
}

#[tauri::command]
pub fn delete_instance(name: String) -> Result<(), String> {
    let config = load_config();
    instance::delete_instance(&config.instances_dir(), &name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launch_instance(name: String) -> Result<String, String> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &name);
    let inst = Instance::load_from(&instance_dir).map_err(|e| e.to_string())?;

    let account = config
        .accounts
        .first()
        .cloned()
        .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

    let meta_path = config
        .versions_dir()
        .join(&inst.minecraft_version)
        .join(format!("{}.json", &inst.minecraft_version));

    let meta_content = std::fs::read_to_string(&meta_path).map_err(|e| e.to_string())?;
    let meta: miao_core::version::meta::VersionMeta =
        serde_json::from_str(&meta_content).map_err(|e| e.to_string())?;

    let required_java = meta.required_java_major();
    let java_installations = java::detect_system_java();
    let java_path = inst.java_path.clone().or_else(|| {
        java::find_compatible_java(&java_installations, required_java).map(|j| j.path.clone())
    });

    let Some(java_path) = java_path else {
        return Err(format!(
            "Java {} not found. Please download it first.",
            required_java
        ));
    };

    let options = LaunchOptions {
        game_dir: instance_dir,
        java_path,
        version_meta: meta,
        instance: inst.clone(),
        auth: account,
        config,
    };

    match build_launch_command(&options) {
        Ok(mut cmd) => {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
            cmd.spawn().map_err(|e| e.to_string())?;
            Ok(format!("Launched {}", name))
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn search_mods(
    query: String,
    mc_version: String,
    loader: Option<String>,
) -> Result<Vec<miao_core::modrinth::api::SearchHit>, String> {
    miao_core::modrinth::api::search_mods(&query, Some(&mc_version), loader.as_deref(), 20)
        .await
        .map(|r| r.hits)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_mod(instance_name: String, project_id: String) -> Result<Vec<String>, String> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    let inst = Instance::load_from(&instance_dir).map_err(|e| e.to_string())?;
    let mods_dir = Instance::mods_dir(&instance_dir);
    let loader = inst
        .mod_loader
        .as_ref()
        .map(|l| l.loader_type.as_str())
        .unwrap_or("fabric");

    let results = miao_core::modrinth::api::install_mod_with_dependencies(
        &project_id,
        &inst.minecraft_version,
        loader,
        &mods_dir,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(results.iter().map(|m| m.filename.clone()).collect())
}

#[tauri::command]
pub fn get_mods(instance_name: String) -> Vec<ModItem> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    let mods_dir = Instance::mods_dir(&instance_dir);
    modmanager::scan_mods_dir(&mods_dir)
        .into_iter()
        .map(|m| ModItem {
            name: m.name.clone(),
            filename: m.file_name.clone(),
            enabled: m.enabled,
        })
        .collect()
}

#[tauri::command]
pub fn toggle_mod(instance_name: String, filename: String) -> Result<bool, String> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    let mods_dir = Instance::mods_dir(&instance_dir);
    let mods = modmanager::scan_mods_dir(&mods_dir);
    let mut m = mods
        .into_iter()
        .find(|m| m.file_name == filename)
        .ok_or("Mod not found")?;
    m.toggle().map_err(|e| e.to_string())?;
    Ok(m.enabled)
}

#[tauri::command]
pub fn delete_mod(instance_name: String, filename: String) -> Result<(), String> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    let mods_dir = Instance::mods_dir(&instance_dir);
    let mods = modmanager::scan_mods_dir(&mods_dir);
    let m = mods
        .into_iter()
        .find(|m| m.file_name == filename)
        .ok_or("Mod not found")?;
    m.delete().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_mrpack(instance_name: String, output_dir: String) -> Result<String, String> {
    let config = load_config();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance_name);
    let inst = Instance::load_from(&instance_dir).map_err(|e| e.to_string())?;
    let output = std::path::PathBuf::from(output_dir);
    miao_core::modrinth::mrpack::export_mrpack(&instance_dir, &inst, &output)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_mrpack(path: String) -> Result<InstanceInfo, String> {
    let config = load_config();
    let mrpack_path = std::path::PathBuf::from(path);
    let inst = miao_core::modrinth::mrpack::import_mrpack(&mrpack_path, &config, None)
        .await
        .map_err(|e| e.to_string())?;
    Ok(InstanceInfo::from(&inst))
}

#[tauri::command]
pub fn get_config() -> ConfigInfo {
    let config = load_config();
    ConfigInfo {
        data_dir: config.data_dir.display().to_string(),
        max_concurrent_downloads: config.max_concurrent_downloads,
        accounts: config
            .accounts
            .iter()
            .map(|a| match a {
                AuthMethod::Offline(acc) => format!("{} (Offline)", acc.username),
                AuthMethod::Microsoft(acc) => format!("{} (Microsoft)", acc.username),
            })
            .collect(),
        active_account: config.active_account_index,
    }
}

#[tauri::command]
pub fn save_config(data_dir: String, max_downloads: usize) -> Result<(), String> {
    let mut config = load_config();
    let new_path = std::path::PathBuf::from(&data_dir);
    let old_path = config.data_dir.clone();

    if new_path != old_path {
        let _ = std::fs::create_dir_all(&new_path);
        if old_path.exists() {
            let subdirs = ["instances", "versions", "libraries", "assets", "java"];
            for dir in &subdirs {
                let src = old_path.join(dir);
                let dst = new_path.join(dir);
                if src.exists() && !dst.exists() {
                    let _ = std::fs::rename(&src, &dst);
                }
            }
        }
        config.data_dir = new_path;
    }

    config.max_concurrent_downloads = max_downloads;
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn detect_java() -> Vec<JavaInfo> {
    java::detect_system_java()
        .into_iter()
        .map(|j| JavaInfo {
            major_version: j.major_version,
            version: j.version,
            path: j.path.display().to_string(),
        })
        .collect()
}

#[tauri::command]
pub async fn download_java(major_version: u32) -> Result<String, String> {
    let config = load_config();
    let http = reqwest::Client::new();
    let asset = miao_core::java::download::fetch_latest_asset(&http, major_version)
        .await
        .map_err(|e| e.to_string())?;
    let java_dir = config.data_dir.join("java");
    let path = miao_core::java::download::download_and_extract_java_with_progress(
        &http,
        &asset,
        &java_dir,
        |_| {},
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn add_offline_account(username: String) -> Result<(), String> {
    let mut config = load_config();
    let account = create_offline_account(&username);
    config.accounts.push(AuthMethod::Offline(account));
    if config.active_account_index.is_none() {
        config.active_account_index = Some(0);
    }
    config.save().map_err(|e| e.to_string())
}
