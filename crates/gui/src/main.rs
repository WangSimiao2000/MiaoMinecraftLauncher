mod app;
pub mod blur;
mod controller;
mod dialogs;
pub mod icons;
mod messages;
pub mod navigation;
pub mod state;
mod theme;
pub mod toast;
mod views;
pub mod widgets;

use anyhow::Result;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([720.0, 480.0])
            .with_title("MMCL")
            .with_decorations(false),
        ..Default::default()
    };

    eframe::run_native(
        "MMCL",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "misans".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/MiSans-Medium.ttf"
                ))),
            );
            fonts.font_data.insert(
                "icons".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/bootstrap-icons.ttf"
                ))),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "misans".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("icons".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("misans".to_owned());

            cc.egui_ctx.set_fonts(fonts);

            egui_extras::install_image_loaders(&cc.egui_ctx);
            let config = miao_core::config::LauncherConfig::load().unwrap_or_default();
            theme::apply_theme(&cc.egui_ctx, config.theme);
            Ok(Box::new(app::MiaoApp::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
