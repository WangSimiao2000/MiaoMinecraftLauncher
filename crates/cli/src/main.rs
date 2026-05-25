mod commands;

use clap::{Parser, Subcommand};
use miao_core::config::LauncherConfig;
use miao_core::service::LauncherService;

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
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let config = LauncherConfig::load().unwrap_or_default();
    let mut service = LauncherService::new(config);

    match cli.command {
        Commands::List => commands::instance::cmd_list(&service)?,
        Commands::Versions { snapshots } => {
            commands::instance::cmd_versions(&service, snapshots).await?
        }
        Commands::New {
            version,
            name,
            loader,
            loader_version,
        } => {
            commands::instance::cmd_new(
                &service,
                &version,
                name.as_deref(),
                loader.as_deref(),
                loader_version.as_deref(),
            )
            .await?
        }
        Commands::Launch { instance } => {
            commands::instance::cmd_launch(&service, &instance).await?
        }
        Commands::Delete { instance } => commands::instance::cmd_delete(&service, &instance)?,
        Commands::Account { username } => {
            commands::instance::cmd_account(&mut service, &username)?
        }
        Commands::Java => commands::java::cmd_java(&service),
        Commands::DownloadJava { instance } => {
            commands::java::cmd_download_java(&service, &instance).await?
        }
        Commands::Mods {
            instance,
            toggle,
            delete,
        } => commands::mods::cmd_mods(&service, &instance, toggle.as_deref(), delete.as_deref())?,
        Commands::Resources { instance, delete } => {
            commands::resources::cmd_resources(&service, &instance, delete.as_deref())?
        }
        Commands::Shaders { instance, delete } => {
            commands::resources::cmd_shaders(&service, &instance, delete.as_deref())?
        }
        Commands::Saves { instance, delete } => {
            commands::resources::cmd_saves(&service, &instance, delete.as_deref())?
        }
        Commands::Open { instance } => commands::instance::cmd_open(&service, &instance)?,
        Commands::Loaders { version } => {
            commands::loaders::cmd_loaders(&service, &version).await?
        }
        Commands::ModSearch {
            query,
            mc_version,
            loader,
        } => {
            commands::mods::cmd_mod_search(&service, &query, mc_version.as_deref(), loader.as_deref())
                .await?
        }
        Commands::ModInstall { instance, project } => {
            commands::mods::cmd_mod_install(&service, &instance, &project).await?
        }
        Commands::Export { instance, output } => {
            commands::instance::cmd_export(&service, &instance, output.as_deref())?
        }
        Commands::Import { path, name } => {
            commands::instance::cmd_import(&service, &path, name.as_deref()).await?
        }
        Commands::UpgradeLoader { instance, version } => {
            commands::loaders::cmd_upgrade_loader(&service, &instance, version.as_deref()).await?
        }
    }

    Ok(())
}
