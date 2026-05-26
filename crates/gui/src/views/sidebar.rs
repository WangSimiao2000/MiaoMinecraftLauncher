use eframe::egui;

use crate::app::{DetailTab, I18n, MiaoApp};
use crate::navigation::Page;
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

                let bottom_height = 40.0;
                let available = ui.available_height() - bottom_height;

                ui.allocate_ui(egui::vec2(ui.available_width(), available), |ui| {
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

                ui.separator();
                ui.add_space(4.0);

                let settings_btn = ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.add_sized(
                        [ui.available_width() - 8.0, 28.0],
                        egui::Button::new(
                            egui::RichText::new(format!("\u{2699}  {}", I18n::t(lang, "settings")))
                                .size(13.0)
                                .color(theme::Colors::text_secondary()),
                        )
                        .frame(false),
                    )
                });
                if settings_btn.inner.clicked() {
                    self.nav_stack.push(Page::Settings);
                }
                ui.add_space(4.0);
            });
    }
}
