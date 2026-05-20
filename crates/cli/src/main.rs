use clap::{Parser, Subcommand};
use miao_core::auth::offline::create_offline_account;
use miao_core::auth::AuthMethod;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{build_launch_command, LaunchOptions};
use miao_core::version::{install, manifest, VersionType};

#[derive(Parser)]
#[command(name = "miao", version, about = "MiaoMinecraftLauncher - A feature-rich Minecraft launcher for Linux")]
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
    /// Install a Minecraft version and create an instance
    Install {
        /// Minecraft version (e.g. 1.20.4)
        version: String,
        /// Instance name (defaults to version)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Launch an instance
    Launch {
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let cli = Cli::parse();
    let config = LauncherConfig::load().unwrap_or_default();

    match cli.command {
        Commands::List => cmd_list(&config)?,
        Commands::Versions { snapshots } => cmd_versions(&config, snapshots).await?,
        Commands::Install { version, name } => cmd_install(&config, &version, name.as_deref()).await?,
        Commands::Launch { instance } => cmd_launch(&config, &instance).await?,
        Commands::Account { username } => cmd_account(&config, &username)?,
        Commands::Java => cmd_java(),
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
        println!("{:<20} {:<12} {}", inst.name, inst.minecraft_version, loader);
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
        versions.iter().filter(|v| v.is_release()).take(20).collect()
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

async fn cmd_install(
    config: &LauncherConfig,
    version: &str,
    name: Option<&str>,
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
    let meta = install::fetch_version_meta(&http, &version_info.url, &config.download_mirror).await?;

    println!("Collecting download tasks...");
    let tasks = install::all_download_tasks(&meta, config, &config.download_mirror);

    let asset_index_task = install::collect_asset_index_download(&meta, config, &config.download_mirror);
    let asset_index_path = asset_index_task.dest.clone();

    println!("Downloading {} files...", tasks.len());
    let dm = DownloadManager::new(config.download_mirror.clone(), config.max_concurrent_downloads);
    dm.download_all(tasks).await?;

    if asset_index_path.exists() {
        println!("Downloading assets...");
        let asset_index = miao_core::version::assets::fetch_asset_index(&asset_index_path).await?;
        let asset_tasks = miao_core::version::assets::collect_asset_downloads(&asset_index, config, &config.download_mirror);
        println!("Downloading {} asset files...", asset_tasks.len());
        let dm2 = DownloadManager::new(config.download_mirror.clone(), config.max_concurrent_downloads);
        dm2.download_all(asset_tasks).await?;
    }

    println!("Creating instance '{}'...", instance_name);
    let inst = Instance::new(instance_name, version);
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    println!("✓ Instance '{}' installed successfully! (MC {})", instance_name, version);
    println!("  Run: miao launch {}", instance_name);

    Ok(())
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
            java::find_compatible_java(&java_installations, required_java)
                .map(|j| j.path.clone())
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No compatible Java {} found. Install Java {} or set java_path in instance config.",
                required_java,
                required_java
            )
        })?;

    println!("Launching {} (MC {}) with Java {}...", inst.name, inst.minecraft_version, java_path.display());

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
    println!("Created offline account: {} (UUID: {})", account.username, account.uuid);

    config.accounts.push(AuthMethod::Offline(account));
    if config.active_account_index.is_none() {
        config.active_account_index = Some(0);
    }
    config.save()?;
    println!("✓ Account saved.");
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
        println!("{:<8} {:<15} {}", j.major_version, j.version, j.path.display());
    }
}
