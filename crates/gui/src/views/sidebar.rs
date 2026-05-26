use eframe::egui;

use crate::app::{DetailTab, I18n, MiaoApp};
use crate::theme;
use crate::widgets::InstanceCard;

impl MiaoApp {
    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        let lang = self.language;
        egui::SidePanel::left("instance_list")
            .resizable(true)
            .default_width(240.0)
            .frame(theme::panel_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(theme::subheading(I18n::t(lang, "instances")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(I18n::t(lang, "import")).clicked() {
                            self.import_with_dialog();
                        }
                        if ui.small_button(I18n::t(lang, "new")).clicked() {
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
                        ui.label(theme::muted(I18n::t(lang, "no_instances")));
                        ui.add_space(8.0);
                        ui.label(theme::small(I18n::t(lang, "no_instances_hint")));
                    });
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..self.instances.len() {
                            let selected = self.selected_instance == Some(i);
                            let inst = &self.instances[i];
                            let loader_label =
                                inst.mod_loader.as_ref().map(|l| l.loader_type.to_string());

                            let response = InstanceCard::new(&inst.name, selected)
                                .loader(loader_label.as_deref())
                                .show(ui);

                            if response.clicked() {
                                self.selected_instance = Some(i);
                                self.active_tab = DetailTab::Mods;
                                self.confirm_delete = None;
                            }
                        }
                    });
                }
            });
    }
}
