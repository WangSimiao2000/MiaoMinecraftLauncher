use eframe::egui;
use std::path::Path;

use crate::app::MiaoApp;
use crate::theme;

impl MiaoApp {
    pub fn render_resources_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let res_dir = miao_core::instance::Instance::resourcepacks_dir(instance_dir);
        let packs = miao_core::resource::scan_resourcepacks(&res_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!(
                "Resource Packs ({})",
                packs.len()
            )));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open folder").clicked() {
                    let _ = miao_core::instance::open_folder(&res_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if packs.is_empty() {
            ui.label(theme::muted("No resource packs"));
        } else {
            for p in &packs {
                ui.horizontal(|ui| {
                    ui.label(theme::body(&p.name));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Del").clicked() {
                            let _ = p.delete();
                        }
                    });
                });
            }
        }

        ui.add_space(theme::Spacing::SECTION_GAP);
        ui.separator();
        ui.add_space(theme::Spacing::SMALL_GAP);

        let shader_dir = miao_core::instance::Instance::shaderpacks_dir(instance_dir);
        let shaders = miao_core::resource::scan_shaderpacks(&shader_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!("Shaders ({})", shaders.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Open folder").clicked() {
                    let _ = miao_core::instance::open_folder(&shader_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if shaders.is_empty() {
            ui.label(theme::muted("No shaders"));
        } else {
            for s in &shaders {
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
