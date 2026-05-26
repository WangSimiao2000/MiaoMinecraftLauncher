mod app;
mod background;
mod controller;
mod dialogs;
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
            .with_title("MMCL"),
        ..Default::default()
    };

    eframe::run_native(
        "MMCL",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();

            fonts.font_data.insert(
                "inter".to_owned(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/Inter-Medium.ttf"
                ))),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "inter".to_owned());

            let cjk_paths: Vec<&str> = match std::env::consts::OS {
                "linux" => vec![
                    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                ],
                "windows" => vec![
                    "C:\\Windows\\Fonts\\msyh.ttc",
                    "C:\\Windows\\Fonts\\simsun.ttc",
                ],
                "macos" => vec![
                    "/System/Library/Fonts/PingFang.ttc",
                    "/Library/Fonts/Arial Unicode.ttf",
                ],
                _ => vec![],
            };
            for path in &cjk_paths {
                if let Ok(data) = std::fs::read(path) {
                    let mut font_data = egui::FontData::from_owned(data);
                    font_data.index = 2;
                    fonts
                        .font_data
                        .insert("cjk".to_owned(), std::sync::Arc::new(font_data));
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .push("cjk".to_owned());
                    break;
                }
            }

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
