use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("MiaoMinecraftLauncher TUI starting...");

    todo!("TUI implementation with ratatui")
}
