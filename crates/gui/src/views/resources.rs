use eframe::egui;
use std::path::Path;

use crate::app::{I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_resources_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        self.file_scan_cache.get_or_scan(instance_dir);
        let res_dir = miao_core::instance::Instance::resourcepacks_dir(instance_dir);
        let packs = self.file_scan_cache.resourcepacks.clone();

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!(
                "{} ({})",
                I18n::t(lang, "tab_resources"),
                packs.len()
            )));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "open")).clicked() {
                    let _ = miao_core::instance::open_folder(&res_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if packs.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(lang, "no_resource_packs")));
                ui.add_space(8.0);
                ui.label(theme::small(I18n::t(lang, "no_resource_packs_hint")));
            });
        } else {
            for p in &packs {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(theme::body(&p.name));
                        let kind = if p.path.is_dir() { "folder" } else { "zip" };
                        ui.label(theme::small(&format!("[{}]", kind)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(I18n::t(lang, "del")).clicked() {
                                let _ = p.delete();
                            }
                            if !p.path.is_dir()
                                && let Ok(meta) = std::fs::metadata(&p.path)
                            {
                                let size_kb = meta.len() as f64 / 1024.0;
                                if size_kb > 1024.0 {
                                    ui.label(theme::small(&format!("{:.1} MB", size_kb / 1024.0)));
                                } else {
                                    ui.label(theme::small(&format!("{:.0} KB", size_kb)));
                                }
                            }
                        });
                    });
                });
                ui.add_space(2.0);
            }
        }

        ui.add_space(theme::Spacing::SECTION_GAP);
        ui.separator();
        ui.add_space(theme::Spacing::SMALL_GAP);

        let shader_dir = miao_core::instance::Instance::shaderpacks_dir(instance_dir);
        let shaders = self.file_scan_cache.shaderpacks.clone();

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!(
                "{} ({})",
                I18n::t(lang, "shaders"),
                shaders.len()
            )));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "open")).clicked() {
                    let _ = miao_core::instance::open_folder(&shader_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if shaders.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(lang, "no_shaders")));
                ui.add_space(8.0);
                ui.label(theme::small(I18n::t(lang, "no_shaders_hint")));
            });
        } else {
            for s in &shaders {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(theme::body(&s.name));
                        let kind = if s.path.is_dir() { "folder" } else { "zip" };
                        ui.label(theme::small(&format!("[{}]", kind)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(I18n::t(lang, "del")).clicked() {
                                let _ = s.delete();
                            }
                            if !s.path.is_dir()
                                && let Ok(meta) = std::fs::metadata(&s.path)
                            {
                                let size_kb = meta.len() as f64 / 1024.0;
                                if size_kb > 1024.0 {
                                    ui.label(theme::small(&format!("{:.1} MB", size_kb / 1024.0)));
                                } else {
                                    ui.label(theme::small(&format!("{:.0} KB", size_kb)));
                                }
                            }
                        });
                    });
                });
                ui.add_space(2.0);
            }
        }
    }
}
