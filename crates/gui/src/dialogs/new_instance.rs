use eframe::egui;

use crate::app::{AsyncState, Dialog, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_new_instance_dialog(&mut self, ctx: &egui::Context, state: &AsyncState) {
        if !state.versions.versions.is_empty()
            && state.loader.versions.is_empty()
            && !state.loader.loading
        {
            let mc_ver = state.versions.versions[self.new_instance.version_idx]
                .id
                .clone();
            self.fetch_loader_versions(&mc_ver);
        }

        let mut open = true;
        egui::Window::new("New Instance")
            .open(&mut open)
            .resizable(false)
            .default_width(450.0)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(theme::subheading("Create New Instance"));
                ui.add_space(theme::Spacing::SMALL_GAP);

                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_instance.name)
                            .vertical_align(egui::Align::Center)
                            .min_size(ui.spacing().interact_size),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("MC Version:");
                    if !state.versions.versions.is_empty() {
                        let current = state
                            .versions
                            .versions
                            .get(self.new_instance.version_idx)
                            .map(|v| v.id.as_str())
                            .unwrap_or("?");
                        let prev_idx = self.new_instance.version_idx;
                        egui::ComboBox::from_id_salt("mc_ver")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (i, ver) in state.versions.versions.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.new_instance.version_idx,
                                        i,
                                        &ver.id,
                                    );
                                }
                            });
                        if self.new_instance.version_idx != prev_idx {
                            self.new_instance.loader = 0;
                            self.new_instance.loader_version_idx = 0;
                            let mc_ver = state.versions.versions[self.new_instance.version_idx]
                                .id
                                .clone();
                            self.fetch_loader_versions(&mc_ver);
                        }
                    } else {
                        ui.spinner();
                        ui.label("Loading versions...");
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Mod Loader:");
                    let loaders = self.get_available_loaders(state);
                    let current_name = loaders
                        .iter()
                        .find(|(idx, _, _)| *idx == self.new_instance.loader)
                        .map(|(_, name, _)| *name)
                        .unwrap_or("None (Vanilla)");

                    egui::ComboBox::from_id_salt("loader")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for (idx, name, available) in &loaders {
                                ui.add_enabled_ui(*available, |ui| {
                                    let label = if *available {
                                        name.to_string()
                                    } else {
                                        format!("{} (N/A)", name)
                                    };
                                    ui.selectable_value(&mut self.new_instance.loader, *idx, label);
                                });
                            }
                        });
                });

                if self.new_instance.loader > 0 {
                    let loader_versions = self.get_loader_versions(state);
                    if !loader_versions.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Loader Version:");
                            let current = loader_versions
                                .get(self.new_instance.loader_version_idx)
                                .map(|v| v.version.as_str())
                                .unwrap_or("?");
                            egui::ComboBox::from_id_salt("loader_ver")
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    for (i, v) in loader_versions.iter().enumerate() {
                                        let label = if v.stable {
                                            format!("{} (stable)", v.version)
                                        } else {
                                            v.version.clone()
                                        };
                                        ui.selectable_value(
                                            &mut self.new_instance.loader_version_idx,
                                            i,
                                            label,
                                        );
                                    }
                                });
                        });
                    } else if state.loader.loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading loader versions...");
                        });
                    }
                }

                ui.add_space(theme::Spacing::SECTION_GAP);

                let can_create = !state.versions.versions.is_empty() && !state.install.installing;
                ui.add_enabled_ui(can_create, |ui| {
                    if ui.button("Create").clicked() {
                        let ver = state.versions.versions[self.new_instance.version_idx].clone();
                        let name = if self.new_instance.name.is_empty() {
                            ver.id.clone()
                        } else {
                            self.new_instance.name.clone()
                        };

                        let loader = if self.new_instance.loader > 0 {
                            let loader_versions = self.get_loader_versions(state);
                            let lt = miao_core::modloader::ModLoaderType::from_index(
                                self.new_instance.loader - 1,
                            );
                            let lv = loader_versions
                                .get(self.new_instance.loader_version_idx)
                                .map(|v| v.version.clone());
                            match (lt, lv) {
                                (Some(lt), Some(lv)) => Some((lt.as_str().to_string(), lv)),
                                _ => None,
                            }
                        } else {
                            None
                        };

                        self.create_instance(ver, name, loader);
                        self.active_dialog = Dialog::None;
                    }
                });
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }
}
