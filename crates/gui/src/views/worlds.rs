use eframe::egui;
use std::path::Path;

use crate::app::MiaoApp;
use crate::theme;

impl MiaoApp {
    pub fn render_worlds_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let saves = miao_core::instance::list_saves(instance_dir);
        let saves_dir = miao_core::instance::Instance::saves_dir(instance_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!("Worlds ({})", saves.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open folder").clicked() {
                    let _ = miao_core::instance::open_folder(&saves_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if saves.is_empty() {
            ui.label(theme::muted(
                "No worlds yet. Launch the game to create one.",
            ));
        } else {
            for s in &saves {
                ui.horizontal(|ui| {
                    ui.label(theme::body(&s.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Del").clicked() {
                            let _ = s.delete();
                        }
                    });
                });
            }
        }
    }
}
