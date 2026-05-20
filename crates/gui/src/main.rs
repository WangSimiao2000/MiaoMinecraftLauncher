use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("MiaoMinecraftLauncher GUI starting...");

    todo!("GUI implementation with egui/eframe")
}
