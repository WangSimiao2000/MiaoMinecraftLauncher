use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "miao", about = "MiaoMinecraftLauncher - A feature-rich Minecraft launcher for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Launch {
        instance: String,
    },
    List,
    Install {
        version: String,
    },
    Tui,
    Gui,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Launch { instance } => {
            println!("Launching instance: {}", instance);
            todo!()
        }
        Commands::List => {
            println!("Listing instances...");
            todo!()
        }
        Commands::Install { version } => {
            println!("Installing Minecraft {}", version);
            todo!()
        }
        Commands::Tui => {
            println!("Starting TUI mode...");
            todo!()
        }
        Commands::Gui => {
            println!("Starting GUI mode...");
            todo!()
        }
    }
}
