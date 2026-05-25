use eframe::egui;
use std::path::Path;

use crate::app::{I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_log_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        let log_path = instance_dir.join("logs").join("latest.log");
        let state = self.async_state.lock().unwrap();
        let has_live_log = !state.game_log.lines.is_empty() || state.game_log.running;
        let live_lines: Vec<String> = state.game_log.lines.iter().cloned().collect();
        let is_running = state.game_log.running;
        drop(state);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(lang, "game_log")));
            if is_running {
                ui.spinner();
                ui.label(theme::small("Running"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "clear_log")).clicked() {
                    self.async_state.lock().unwrap().game_log.lines.clear();
                }
                if log_path.exists() && ui.button("Open file").clicked() {
                    let _ = open::that(&log_path);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if has_live_log {
            egui::Frame::none()
                .fill(theme::Colors::BG_DARK)
                .rounding(theme::Radii::WIDGET)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(500.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in &live_lines {
                                let color = if line.starts_with("[ERR]") {
                                    theme::Colors::DANGER
                                } else if line.contains("WARN") {
                                    theme::Colors::WARNING
                                } else {
                                    theme::Colors::TEXT_SECONDARY
                                };
                                ui.label(
                                    egui::RichText::new(line)
                                        .monospace()
                                        .size(11.0)
                                        .color(color),
                                );
                            }
                        });
                });
        } else if log_path.exists() {
            let content = std::fs::read_to_string(&log_path).unwrap_or_default();
            let lines: Vec<&str> = content.lines().collect();
            let tail_start = lines.len().saturating_sub(100);
            let tail = &lines[tail_start..];

            egui::Frame::none()
                .fill(theme::Colors::BG_DARK)
                .rounding(theme::Radii::WIDGET)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(500.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for line in tail {
                                ui.label(
                                    egui::RichText::new(*line)
                                        .monospace()
                                        .size(11.0)
                                        .color(theme::Colors::TEXT_SECONDARY),
                                );
                            }
                        });
                });
        } else {
            ui.label(theme::muted(I18n::t(lang, "no_log")));
        }
    }
}
