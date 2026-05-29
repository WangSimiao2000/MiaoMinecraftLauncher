// On Windows, hide the console window for release builds so the GUI doesn't
// flash a black cmd window on startup. Debug builds keep the console so
// `tracing` output and panics stay visible while developing. No effect on
// other platforms.
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

pub mod animation;
mod app;
pub mod blur;
mod controller;
mod dialogs;
pub mod icons;
mod logging;
mod messages;
pub mod navigation;
pub mod platform;
pub mod state;
mod theme;
pub mod toast;
mod views;
pub mod widgets;

use anyhow::Result;

fn main() -> Result<()> {
    platform::enable_high_dpi_awareness();

    let bootstrap_config = miao_core::config::LauncherConfig::load().unwrap_or_default();
    let _log_guard = logging::init(&bootstrap_config.logs_dir());
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        "MMCL starting"
    );

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([960.0, 640.0])
        .with_min_inner_size([720.0, 480.0])
        .with_title("MMCL");
    let options = eframe::NativeOptions {
        viewport: platform::apply_window_chrome(viewport),
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

            if let Some(cjk_fallback) = find_system_fallback_font() {
                fonts.font_data.insert(
                    "cjk-fallback".to_owned(),
                    std::sync::Arc::new(egui::FontData::from_owned(cjk_fallback)),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("cjk-fallback".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("cjk-fallback".to_owned());
            }

            cc.egui_ctx.set_fonts(fonts);

            egui_extras::install_image_loaders(&cc.egui_ctx);
            let config = miao_core::config::LauncherConfig::load().unwrap_or_default();
            theme::apply_theme(&cc.egui_ctx, config.theme);
            Ok(Box::new(app::MiaoApp::new(cc)))
        }),
    )
    .map_err(|e| {
        tracing::error!(error = %e, "eframe runtime exited with error");
        anyhow::anyhow!("eframe error: {}", e)
    })?;

    tracing::info!("MMCL exiting cleanly");
    Ok(())
}

fn find_system_fallback_font() -> Option<Vec<u8>> {
    for path in platform::system_fallback_font_paths() {
        if let Ok(data) = std::fs::read(path) {
            return Some(data);
        }
    }
    None
}
