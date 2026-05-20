use clap::{Parser, Subcommand};
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::version::{VersionType, install, manifest};

#[derive(Parser)]
#[command(
    name = "miao",
    version,
    about = "MiaoMinecraftLauncher - A feature-rich Minecraft launcher for Linux"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all instances
    List,
    /// List available Minecraft versions
    Versions {
        /// Show snapshots too
        #[arg(short, long)]
        snapshots: bool,
    },
    /// Create a new instance (MC version + optional mod loader)
    New {
        /// Minecraft version (e.g. 1.20.4)
        version: String,
        /// Instance name (defaults to version)
        #[arg(short, long)]
        name: Option<String>,
        /// Mod loader: fabric, quilt, neoforge, forge
        #[arg(short, long)]
        loader: Option<String>,
        /// Mod loader version (uses latest stable if omitted)
        #[arg(long)]
        loader_version: Option<String>,
    },
    /// Launch an instance
    Launch {
        /// Instance name
        instance: String,
    },
    /// Delete an instance
    Delete {
        /// Instance name
        instance: String,
    },
    /// Add an offline account
    Account {
        /// Username for offline account
        username: String,
    },
    /// Detect installed Java versions
    Java,
    /// Download required Java for an instance from Adoptium
    DownloadJava {
        /// Instance name
        instance: String,
    },
    /// Manage mods for an instance (list/toggle/delete)
    Mods {
        /// Instance name
        instance: String,
        /// Toggle a mod by name (enable/disable)
        #[arg(short, long)]
        toggle: Option<String>,
        /// Delete a mod by name
        #[arg(short, long)]
        delete: Option<String>,
    },
    /// Manage resource packs for an instance (list/delete)
    Resources {
        /// Instance name
        instance: String,
        /// Delete a resource pack by name
        #[arg(short, long)]
        delete: Option<String>,
    },
    /// Manage shader packs for an instance (list/delete)
    Shaders {
        /// Instance name
        instance: String,
        /// Delete a shader pack by name
        #[arg(short, long)]
        delete: Option<String>,
    },
    /// Manage save worlds for an instance (list/delete)
    Saves {
        /// Instance name
        instance: String,
        /// Delete a world by name
        #[arg(short, long)]
        delete: Option<String>,
    },
    /// Open instance folder in file manager
    Open {
        /// Instance name
        instance: String,
    },
    /// Show compatible mod loaders for a MC version
    Loaders {
        /// Minecraft version (e.g. 1.20.4)
        version: String,
    },
    /// Search mods on Modrinth
    ModSearch {
        /// Search query
        query: String,
        /// Filter by MC version
        #[arg(short = 'v', long)]
        mc_version: Option<String>,
        /// Filter by loader (fabric, quilt, neoforge, forge)
        #[arg(short, long)]
        loader: Option<String>,
    },
    /// Install a mod from Modrinth into an instance
    ModInstall {
        /// Instance name
        instance: String,
        /// Project ID or slug from Modrinth
        project: String,
    },
    /// Export instance as .mrpack
    Export {
        /// Instance name
        instance: String,
        /// Output path (default: current directory)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Import a .mrpack modpack
    Import {
        /// Path to .mrpack file
        path: String,
        /// Instance name (defaults to pack name)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Upgrade the mod loader version for an instance (same loader type only)
    UpgradeLoader {
        /// Instance name
        instance: String,
        /// Target loader version (uses latest stable if omitted)
        #[arg(short, long)]
        version: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let cli = Cli::parse();
    let config = LauncherConfig::load().unwrap_or_default();

    match cli.command {
        Commands::List => cmd_list(&config)?,
        Commands::Versions { snapshots } => cmd_versions(&config, snapshots).await?,
        Commands::New {
            version,
            name,
            loader,
            loader_version,
        } => {
            cmd_new(
                &config,
                &version,
                name.as_deref(),
                loader.as_deref(),
                loader_version.as_deref(),
            )
            .await?
        }
        Commands::Launch { instance } => cmd_launch(&config, &instance).await?,
        Commands::Delete { instance } => cmd_delete(&config, &instance)?,
        Commands::Account { username } => cmd_account(&config, &username)?,
        Commands::Java => cmd_java(),
        Commands::DownloadJava { instance } => cmd_download_java(&config, &instance).await?,
        Commands::Mods {
            instance,
            toggle,
            delete,
        } => cmd_mods(&config, &instance, toggle.as_deref(), delete.as_deref())?,
        Commands::Resources { instance, delete } => {
            cmd_resources(&config, &instance, delete.as_deref())?
        }
        Commands::Shaders { instance, delete } => {
            cmd_shaders(&config, &instance, delete.as_deref())?
        }
        Commands::Saves { instance, delete } => cmd_saves(&config, &instance, delete.as_deref())?,
        Commands::Open { instance } => cmd_open(&config, &instance)?,
        Commands::Loaders { version } => cmd_loaders(&config, &version).await?,
        Commands::ModSearch {
            query,
            mc_version,
            loader,
        } => cmd_mod_search(&query, mc_version.as_deref(), loader.as_deref()).await?,
        Commands::ModInstall { instance, project } => {
            cmd_mod_install(&config, &instance, &project).await?
        }
        Commands::Export { instance, output } => cmd_export(&config, &instance, output.as_deref())?,
        Commands::Import { path, name } => cmd_import(&config, &path, name.as_deref()).await?,
        Commands::UpgradeLoader { instance, version } => {
            cmd_upgrade_loader(&config, &instance, version.as_deref()).await?
        }
    }

    Ok(())
}

fn cmd_list(config: &LauncherConfig) -> anyhow::Result<()> {
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

async fn cmd_versions(config: &LauncherConfig, show_snapshots: bool) -> anyhow::Result<()> {
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

async fn cmd_new(
    config: &LauncherConfig,
    version: &str,
    name: Option<&str>,
    loader: Option<&str>,
    loader_version: Option<&str>,
) -> anyhow::Result<()> {
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
) -> anyhow::Result<miao_core::instance::ModLoaderConfig> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

    match loader_type {
        "fabric" => {
            let versions = fabric::fetch_loader_versions(http, mc_version).await?;
            let ver = match loader_version {
                Some(v) => v.to_string(),
                None => versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Fabric versions for {}", mc_version))?,
            };
            println!("Installing Fabric {}...", ver);
            let profile = fabric::fetch_profile(http, mc_version, &ver).await?;
            let tasks = fabric::collect_fabric_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            Ok(miao_core::instance::ModLoaderConfig {
                loader_type: ModLoaderType::Fabric,
                version: ver,
            })
        }
        "quilt" => {
            let versions = quilt::fetch_loader_versions(http, mc_version).await?;
            let ver = match loader_version {
                Some(v) => v.to_string(),
                None => versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Quilt versions for {}", mc_version))?,
            };
            println!("Installing Quilt {}...", ver);
            let profile = quilt::fetch_profile(http, mc_version, &ver).await?;
            let tasks = quilt::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            Ok(miao_core::instance::ModLoaderConfig {
                loader_type: ModLoaderType::Quilt,
                version: ver,
            })
        }
        "neoforge" => {
            let versions = neoforge::fetch_versions(http, mc_version).await?;
            let ver = match loader_version {
                Some(v) => v.to_string(),
                None => versions
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No NeoForge versions for {}", mc_version))?,
            };
            println!("Installing NeoForge {}...", ver);
            let profile = neoforge::fetch_profile(http, &ver).await?;
            let tasks = neoforge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            Ok(miao_core::instance::ModLoaderConfig {
                loader_type: ModLoaderType::NeoForge,
                version: ver,
            })
        }
        "forge" => {
            let ver = match loader_version {
                Some(v) => v.to_string(),
                None => forge::fetch_recommended_version(http, mc_version)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("No Forge versions for {}", mc_version))?,
            };
            println!("Installing Forge {}...", ver);
            let profile = forge::fetch_install_profile(http, mc_version, &ver).await?;
            let tasks = forge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            Ok(miao_core::instance::ModLoaderConfig {
                loader_type: ModLoaderType::Forge,
                version: ver,
            })
        }
        _ => anyhow::bail!(
            "Unknown loader '{}'. Use: fabric, quilt, neoforge, forge",
            loader_type
        ),
    }
}

async fn cmd_launch(config: &LauncherConfig, instance_name: &str) -> anyhow::Result<()> {
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

fn cmd_account(config: &LauncherConfig, username: &str) -> anyhow::Result<()> {
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

fn cmd_delete(config: &LauncherConfig, instance_name: &str) -> anyhow::Result<()> {
    instance::delete_instance(&config.instances_dir(), instance_name)?;
    println!("✓ Deleted instance '{}'", instance_name);
    Ok(())
}

async fn cmd_download_java(config: &LauncherConfig, instance_name: &str) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let meta_path = config
        .versions_dir()
        .join(&inst.minecraft_version)
        .join(format!("{}.json", &inst.minecraft_version));

    if !meta_path.exists() {
        anyhow::bail!("Version metadata not found for {}", inst.minecraft_version);
    }

    let meta_content = std::fs::read_to_string(&meta_path)?;
    let meta: miao_core::version::meta::VersionMeta = serde_json::from_str(&meta_content)?;
    let required = meta.required_java_major();

    let java_installations = java::detect_system_java();
    if java::find_compatible_java(&java_installations, required).is_some() {
        println!("✓ Java {} already available.", required);
        return Ok(());
    }

    println!(
        "Java {} required but not found. Downloading from Adoptium...",
        required
    );

    let http = reqwest::Client::new();
    let asset = miao_core::java::download::fetch_latest_asset(&http, required).await?;
    let total_mb = asset.binary.package.size as f64 / 1_000_000.0;
    println!("Downloading {} ({:.1} MB)...", asset.release_name, total_mb);

    let java_dir = config.data_dir.join("java");
    let java_bin = miao_core::java::download::download_and_extract_java_with_progress(
        &http,
        &asset,
        &java_dir,
        |phase| {
            use miao_core::java::download::DownloadPhase;
            match phase {
                DownloadPhase::Downloading { downloaded, total } => {
                    eprint!(
                        "\r  {:.1}/{:.1} MB",
                        downloaded as f64 / 1_000_000.0,
                        total as f64 / 1_000_000.0
                    );
                }
                DownloadPhase::Extracting => {
                    eprintln!("\r  Extracting...          ");
                }
            }
        },
    )
    .await?;

    println!("\n✓ Java {} installed at {}", required, java_bin.display());
    Ok(())
}

fn cmd_mods(
    config: &LauncherConfig,
    instance_name: &str,
    toggle: Option<&str>,
    delete: Option<&str>,
) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let mods_dir = Instance::mods_dir(&instance_dir);
    let mut mods = miao_core::modmanager::scan_mods_dir(&mods_dir);

    if let Some(name) = toggle {
        let m = mods
            .iter_mut()
            .find(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("Mod '{}' not found", name))?;
        m.toggle()?;
        let state = if m.enabled { "enabled" } else { "disabled" };
        println!("✓ {} {}", name, state);
        return Ok(());
    }

    if let Some(name) = delete {
        let m = mods
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("Mod '{}' not found", name))?;
        m.delete()?;
        println!("✓ Deleted mod '{}'", name);
        return Ok(());
    }

    if mods.is_empty() {
        println!("No mods installed in '{}'.", instance_name);
    } else {
        println!("{:<6} NAME", "STATE");
        println!("{}", "-".repeat(40));
        for m in &mods {
            let state = if m.enabled { "  ✓" } else { "  ✗" };
            println!("{:<6} {}", state, m.name);
        }
    }
    Ok(())
}

fn cmd_resources(
    config: &LauncherConfig,
    instance_name: &str,
    delete: Option<&str>,
) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let dir = Instance::resourcepacks_dir(&instance_dir);
    let packs = miao_core::resource::scan_resourcepacks(&dir);

    if let Some(name) = delete {
        let p = packs
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("Resource pack '{}' not found", name))?;
        p.delete()?;
        println!("✓ Deleted resource pack '{}'", name);
        return Ok(());
    }

    if packs.is_empty() {
        println!("No resource packs in '{}'.", instance_name);
    } else {
        for p in &packs {
            println!("  {}", p.name);
        }
    }
    Ok(())
}

fn cmd_shaders(
    config: &LauncherConfig,
    instance_name: &str,
    delete: Option<&str>,
) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let dir = Instance::shaderpacks_dir(&instance_dir);
    let shaders = miao_core::resource::scan_shaderpacks(&dir);

    if let Some(name) = delete {
        let s = shaders
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Shader pack '{}' not found", name))?;
        s.delete()?;
        println!("✓ Deleted shader pack '{}'", name);
        return Ok(());
    }

    if shaders.is_empty() {
        println!("No shader packs in '{}'.", instance_name);
    } else {
        for s in &shaders {
            println!("  {}", s.name);
        }
    }
    Ok(())
}

fn cmd_saves(
    config: &LauncherConfig,
    instance_name: &str,
    delete: Option<&str>,
) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let saves = instance::list_saves(&instance_dir);

    if let Some(name) = delete {
        let s = saves
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("World '{}' not found", name))?;
        s.delete()?;
        println!("✓ Deleted world '{}'", name);
        return Ok(());
    }

    if saves.is_empty() {
        println!("No worlds in '{}'.", instance_name);
    } else {
        for s in &saves {
            println!("  {}", s.name);
        }
    }
    Ok(())
}

fn cmd_open(config: &LauncherConfig, instance_name: &str) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    instance::open_folder(&instance_dir)?;
    println!("Opened: {}", instance_dir.display());
    Ok(())
}

async fn cmd_loaders(_config: &LauncherConfig, mc_version: &str) -> anyhow::Result<()> {
    let http = reqwest::Client::new();
    println!("Fetching loader compatibility for {}...", mc_version);

    let versions = miao_core::modloader::fetch_all_loader_versions(&http, mc_version).await?;

    if versions.is_empty() {
        println!("No mod loaders available for {}.", mc_version);
        return Ok(());
    }

    for lt in &miao_core::modloader::ModLoaderType::ALL {
        if let Some(loader_versions) = versions.get(lt) {
            let stable_count = loader_versions.iter().filter(|v| v.stable).count();
            let latest = &loader_versions[0].version;
            println!(
                "  {} — {} versions ({} stable), latest: {}",
                lt,
                loader_versions.len(),
                stable_count,
                latest
            );
        } else {
            println!("  {} — not available", lt);
        }
    }
    Ok(())
}

fn cmd_java() {
    let installations = java::detect_system_java();

    if installations.is_empty() {
        println!("No Java installations found in standard paths.");
        println!("Searched: /usr/lib/jvm, /usr/local/lib/jvm, /usr/java");
        return;
    }

    println!("{:<8} {:<15} PATH", "MAJOR", "VERSION");
    println!("{}", "-".repeat(60));
    for j in &installations {
        println!(
            "{:<8} {:<15} {}",
            j.major_version,
            j.version,
            j.path.display()
        );
    }
}

async fn cmd_mod_search(
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> anyhow::Result<()> {
    let result = miao_core::modrinth::api::search_mods(query, mc_version, loader, 15).await?;

    if result.hits.is_empty() {
        println!("No mods found for '{}'.", query);
        return Ok(());
    }

    println!("{:<30} {:<15} DOWNLOADS", "NAME", "ID");
    println!("{}", "-".repeat(60));
    for hit in &result.hits {
        println!(
            "{:<30} {:<15} {}",
            truncate(&hit.title, 28),
            hit.slug,
            format_downloads(hit.downloads)
        );
    }
    println!("\nInstall: miao mod-install <instance> <slug>");
    Ok(())
}

async fn cmd_mod_install(
    config: &LauncherConfig,
    instance_name: &str,
    project: &str,
) -> anyhow::Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let loader = inst
        .mod_loader
        .as_ref()
        .map(|l| l.loader_type.as_str())
        .unwrap_or("fabric");

    println!(
        "Searching versions for '{}' (MC {}, {})...",
        project, inst.minecraft_version, loader
    );

    let versions = miao_core::modrinth::api::get_project_versions(
        project,
        Some(&inst.minecraft_version),
        Some(loader),
    )
    .await?;

    let version = versions
        .first()
        .ok_or_else(|| anyhow::anyhow!("No compatible version found for {}", project))?;

    let file = version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
        .ok_or_else(|| anyhow::anyhow!("No files in version"))?;

    println!("Installing {} ({})...", version.name, file.filename);

    let mods_dir = Instance::mods_dir(&instance_dir);
    let dest = miao_core::modrinth::api::download_mod_file(file, &mods_dir).await?;

    println!("✓ Installed at {}", dest.display());
    Ok(())
}

fn cmd_export(
    config: &LauncherConfig,
    instance_name: &str,
    output: Option<&str>,
) -> anyhow::Result<()> {
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

async fn cmd_import(config: &LauncherConfig, path: &str, name: Option<&str>) -> anyhow::Result<()> {
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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{}…", truncated)
    }
}

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

async fn cmd_upgrade_loader(
    config: &LauncherConfig,
    instance_name: &str,
    target_version: Option<&str>,
) -> anyhow::Result<()> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let mut inst = Instance::load_from(&instance_dir)?;
    let mc_version = inst.minecraft_version.clone();
    let http = reqwest::Client::new();

    let Some(current_loader) = &inst.mod_loader else {
        anyhow::bail!(
            "Instance '{}' has no mod loader installed. Use 'miao new' with --loader to create a modded instance.",
            instance_name
        );
    };

    let current_type = current_loader.loader_type.clone();
    let current_ver = current_loader.version.clone();

    let new_ver = match &current_type {
        ModLoaderType::Fabric => {
            let versions = fabric::fetch_loader_versions(&http, &mc_version).await?;
            let ver = match target_version {
                Some(v) => v.to_string(),
                None => versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Fabric versions available"))?,
            };
            println!("Upgrading Fabric {} → {}...", current_ver, ver);
            let profile = fabric::fetch_profile(&http, &mc_version, &ver).await?;
            let tasks = fabric::collect_fabric_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            ver
        }
        ModLoaderType::Quilt => {
            let versions = quilt::fetch_loader_versions(&http, &mc_version).await?;
            let ver = match target_version {
                Some(v) => v.to_string(),
                None => versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Quilt versions available"))?,
            };
            println!("Upgrading Quilt {} → {}...", current_ver, ver);
            let profile = quilt::fetch_profile(&http, &mc_version, &ver).await?;
            let tasks = quilt::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            ver
        }
        ModLoaderType::NeoForge => {
            let versions = neoforge::fetch_versions(&http, &mc_version).await?;
            let ver = match target_version {
                Some(v) => v.to_string(),
                None => versions
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No NeoForge versions available"))?,
            };
            println!("Upgrading NeoForge {} → {}...", current_ver, ver);
            let profile = neoforge::fetch_profile(&http, &ver).await?;
            let tasks = neoforge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            ver
        }
        ModLoaderType::Forge => {
            let ver = match target_version {
                Some(v) => v.to_string(),
                None => forge::fetch_recommended_version(&http, &mc_version)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("No Forge versions available"))?,
            };
            println!("Upgrading Forge {} → {}...", current_ver, ver);
            let profile = forge::fetch_install_profile(&http, &mc_version, &ver).await?;
            let tasks = forge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            ver
        }
    };

    inst.mod_loader = Some(miao_core::instance::ModLoaderConfig {
        loader_type: current_type.clone(),
        version: new_ver.clone(),
    });
    inst.save_to(&instance_dir)?;

    println!(
        "✓ {} upgraded to {} for '{}'",
        current_type, new_ver, instance_name
    );
    Ok(())
}
