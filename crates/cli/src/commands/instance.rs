use anyhow::Result;
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::version::{VersionType, install, manifest};

pub fn cmd_list(config: &LauncherConfig) -> Result<()> {
    let instances = instance::list_instances(&config.instances_dir())?;

    if instances.is_empty() {
        println!("No instances found. Use 'miao install <version>' to create one.");
        return Ok(());
    }

    println!("{:<20} {:<12} MOD LOADER", "NAME", "MC VERSION");
    println!("{}", "-".repeat(50));
    for inst in &instances {
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| format!("{} {}", l.loader_type, l.version))
            .unwrap_or_else(|| "Vanilla".to_string());
        println!(
            "{:<20} {:<12} {}",
            inst.name, inst.minecraft_version, loader
        );
    }

    Ok(())
}

pub async fn cmd_versions(config: &LauncherConfig, show_snapshots: bool) -> Result<()> {
    let http = reqwest::Client::new();
    println!("Fetching version manifest...");
    let versions = manifest::fetch_version_manifest(&http, &config.download_mirror).await?;

    let filtered: Vec<_> = if show_snapshots {
        versions.iter().take(30).collect()
    } else {
        versions
            .iter()
            .filter(|v| v.is_release())
            .take(20)
            .collect()
    };

    println!("{:<16} {:<10} RELEASE DATE", "VERSION", "TYPE");
    println!("{}", "-".repeat(50));
    for v in filtered {
        let type_str = match v.version_type {
            VersionType::Release => "release",
            VersionType::Snapshot => "snapshot",
            VersionType::OldBeta => "old_beta",
            VersionType::OldAlpha => "old_alpha",
        };
        println!("{:<16} {:<10} {}", v.id, type_str, v.release_time);
    }

    Ok(())
}

pub async fn cmd_new(
    config: &LauncherConfig,
    version: &str,
    name: Option<&str>,
    loader: Option<&str>,
    loader_version: Option<&str>,
) -> Result<()> {
    let instance_name = name.unwrap_or(version);
    let http = reqwest::Client::new();

    println!("Fetching version manifest...");
    let versions = manifest::fetch_version_manifest(&http, &config.download_mirror).await?;

    let version_info = versions
        .iter()
        .find(|v| v.id == version)
        .ok_or_else(|| anyhow::anyhow!("Version '{}' not found", version))?;

    println!("Downloading version metadata for {}...", version);
    let meta =
        install::fetch_version_meta(&http, &version_info.url, &config.download_mirror).await?;

    install::save_version_meta(&meta, config)?;

    println!("Collecting download tasks...");
    let tasks = install::all_download_tasks(&meta, config, &config.download_mirror);

    let asset_index_task =
        install::collect_asset_index_download(&meta, config, &config.download_mirror);
    let asset_index_path = asset_index_task.dest.clone();

    println!("Downloading {} files...", tasks.len());
    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    dm.download_all(tasks).await?;

    if asset_index_path.exists() {
        println!("Downloading assets...");
        let asset_index = miao_core::version::assets::fetch_asset_index(&asset_index_path).await?;
        let asset_tasks = miao_core::version::assets::collect_asset_downloads(
            &asset_index,
            config,
            &config.download_mirror,
        );
        println!("Downloading {} asset files...", asset_tasks.len());
        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks).await?;
    }

    println!("Creating instance '{}'...", instance_name);
    let mut inst = Instance::new(instance_name, version);

    if let Some(loader_type) = loader {
        let loader_config =
            install_loader_for_new_instance(&http, config, version, loader_type, loader_version)
                .await?;
        inst.mod_loader = Some(loader_config);
    }

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    let loader_info = inst
        .mod_loader
        .as_ref()
        .map(|l| format!(" + {} {}", l.loader_type, l.version))
        .unwrap_or_default();

    println!(
        "✓ Instance '{}' created! (MC {}{})",
        instance_name, version, loader_info
    );
    println!("  Run: miao launch {}", instance_name);

    Ok(())
}

async fn install_loader_for_new_instance(
    http: &reqwest::Client,
    config: &LauncherConfig,
    mc_version: &str,
    loader_type: &str,
    loader_version: Option<&str>,
) -> Result<miao_core::instance::ModLoaderConfig> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

    let lt = match loader_type {
        "fabric" => ModLoaderType::Fabric,
        "quilt" => ModLoaderType::Quilt,
        "neoforge" => ModLoaderType::NeoForge,
        "forge" => ModLoaderType::Forge,
        _ => anyhow::bail!(
            "Unknown loader '{}'. Use: fabric, quilt, neoforge, forge",
            loader_type
        ),
    };

    let ver = match loader_version {
        Some(v) => v.to_string(),
        None => match &lt {
            ModLoaderType::Fabric => {
                let versions = fabric::fetch_loader_versions(http, mc_version).await?;
                versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Fabric versions for {}", mc_version))?
            }
            ModLoaderType::Quilt => {
                let versions = quilt::fetch_loader_versions(http, mc_version).await?;
                versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Quilt versions for {}", mc_version))?
            }
            ModLoaderType::NeoForge => {
                let versions = neoforge::fetch_versions(http, mc_version).await?;
                versions
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No NeoForge versions for {}", mc_version))?
            }
            ModLoaderType::Forge => forge::fetch_recommended_version(http, mc_version)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No Forge versions for {}", mc_version))?,
        },
    };

    println!("Installing {} {}...", loader_type, ver);
    miao_core::modloader::install_loader(http, &lt, mc_version, &ver, config).await
}

pub async fn cmd_launch(config: &LauncherConfig, instance_name: &str) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let account = config
        .accounts
        .first()
        .cloned()
        .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

    let java_installations = java::detect_system_java();
    let version_meta_path = config
        .versions_dir()
        .join(&inst.minecraft_version)
        .join(format!("{}.json", &inst.minecraft_version));

    if !version_meta_path.exists() {
        anyhow::bail!(
            "Version metadata not found for {}. Run 'miao install {}' first.",
            inst.minecraft_version,
            inst.minecraft_version
        );
    }

    let meta_content = std::fs::read_to_string(&version_meta_path)?;
    let meta: miao_core::version::meta::VersionMeta = serde_json::from_str(&meta_content)?;

    let required_java = meta.required_java_major();
    let java_path = inst
        .java_path
        .clone()
        .or_else(|| {
            java::find_compatible_java(&java_installations, required_java).map(|j| j.path.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No compatible Java {} found. Install Java {} or set java_path in instance config.",
                required_java,
                required_java
            )
        })?;

    println!(
        "Launching {} (MC {}) with Java {}...",
        inst.name,
        inst.minecraft_version,
        java_path.display()
    );

    let options = LaunchOptions {
        game_dir: instance_dir.clone(),
        java_path,
        version_meta: meta,
        instance: inst,
        auth: account,
        config: config.clone(),
    };

    let mut cmd = build_launch_command(&options)?;
    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Minecraft exited with code: {:?}", status.code());
    }

    Ok(())
}

pub fn cmd_delete(config: &LauncherConfig, instance_name: &str) -> Result<()> {
    instance::delete_instance(&config.instances_dir(), instance_name)?;
    println!("✓ Deleted instance '{}'", instance_name);
    Ok(())
}

pub fn cmd_open(config: &LauncherConfig, instance_name: &str) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    instance::open_folder(&instance_dir)?;
    println!("Opened: {}", instance_dir.display());
    Ok(())
}

pub fn cmd_account(config: &LauncherConfig, username: &str) -> Result<()> {
    let mut config = config.clone();
    let account = create_offline_account(username);
    println!(
        "Created offline account: {} (UUID: {})",
        account.username, account.uuid
    );

    config.accounts.push(AuthMethod::Offline(account));
    if config.active_account_index.is_none() {
        config.active_account_index = Some(0);
    }
    config.save()?;
    println!("✓ Account saved.");
    Ok(())
}

pub fn cmd_export(
    config: &LauncherConfig,
    instance_name: &str,
    output: Option<&str>,
) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let output_path = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("Exporting '{}' as .mrpack...", instance_name);
    let result = miao_core::modrinth::mrpack::export_mrpack(&instance_dir, &inst, &output_path)?;
    println!("✓ Exported to {}", result.display());
    Ok(())
}

pub async fn cmd_import(config: &LauncherConfig, path: &str, name: Option<&str>) -> Result<()> {
    let mrpack_path = std::path::Path::new(path);
    if !mrpack_path.exists() {
        anyhow::bail!("File not found: {}", path);
    }

    println!("Importing {}...", path);
    let inst = miao_core::modrinth::mrpack::import_mrpack(mrpack_path, config, name).await?;

    let loader_info = inst
        .mod_loader
        .as_ref()
        .map(|l| format!(" + {} {}", l.loader_type, l.version))
        .unwrap_or_default();

    println!(
        "✓ Imported '{}' (MC {}{})",
        inst.name, inst.minecraft_version, loader_info
    );
    Ok(())
}
