use eframe::egui;

use crate::app::{DetailTab, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("instance_list")
            .resizable(true)
            .default_width(240.0)
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::subheading("Instances"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Import").clicked() {
                            self.import_with_dialog();
                        }
                        if ui.small_button("+ New").clicked() {
                            self.open_new_instance_dialog();
                        }
                    });
                });
                ui.add_space(theme::Spacing::SMALL_GAP);
                ui.separator();
                ui.add_space(theme::Spacing::SMALL_GAP);

                if self.instances.is_empty() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(theme::muted("No instances yet"));
                        ui.add_space(8.0);
                        ui.label(theme::small("Click '+ New' to create one"));
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.instances.len() {
                            let selected = self.selected_instance == Some(i);
                            let inst = &self.instances[i];
                            let loader_badge = inst
                                .mod_loader
                                .as_ref()
                                .map(|l| format!(" [{}]", l.loader_type))
                                .unwrap_or_default();
                            let label = format!("{}{}", inst.name, loader_badge);
                            let text = if selected {
                                egui::RichText::new(&label).color(egui::Color32::WHITE)
                            } else {
                                egui::RichText::new(&label).color(theme::Colors::TEXT_PRIMARY)
                            };
                            if ui.selectable_label(selected, text).clicked() {
                                self.selected_instance = Some(i);
                                self.active_tab = DetailTab::Mods;
                            }
                        }
                    });
                }
            });
    }
}
