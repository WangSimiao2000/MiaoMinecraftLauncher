mod app;
mod dialogs;
mod theme;
mod views;

use anyhow::Result;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("MiaoMC"),
        ..Default::default()
    };

    eframe::run_native(
        "MiaoMC",
        options,
        Box::new(|cc| {
            theme::apply_global_style(&cc.egui_ctx);
            Ok(Box::new(app::MiaoApp::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
