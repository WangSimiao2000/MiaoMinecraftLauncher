use eframe::egui;
use std::path::Path;

use crate::app::MiaoApp;
use crate::theme;

impl MiaoApp {
    pub fn render_log_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let log_path = instance_dir.join("logs").join("latest.log");

        ui.horizontal(|ui| {
            ui.label(theme::subheading("Game Log"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if log_path.exists() && ui.button("Open in editor").clicked() {
                    let _ = open::that(&log_path);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if !log_path.exists() {
            ui.label(theme::muted("No log file yet. Launch the game first."));
            return;
        }

        let content = std::fs::read_to_string(&log_path).unwrap_or_default();
        let lines: Vec<&str> = content.lines().collect();
        let tail_start = lines.len().saturating_sub(80);
        let tail = &lines[tail_start..];

        egui::Frame::none()
            .fill(theme::Colors::BG_DARK)
            .rounding(theme::Radii::WIDGET)
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in tail {
                            ui.monospace(*line);
                        }
                    });
            });
    }
}
