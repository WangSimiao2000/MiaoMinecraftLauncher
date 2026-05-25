use eframe::egui;

use crate::app::{DetailTab, I18n, MiaoApp};
use crate::theme;

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

                            let response = ui
                                .scope(|ui| {
                                    let rect = ui.available_rect_before_wrap();
                                    let sense = egui::Sense::click();
                                    let (rect, response) =
                                        ui.allocate_at_least(egui::vec2(rect.width(), 36.0), sense);

                                    let hovered = response.hovered();
                                    let frame = theme::list_item_frame(hovered, selected);
                                    let painter = ui.painter_at(rect);
                                    let visuals = frame.fill;
                                    painter.rect_filled(rect, theme::LIST_ITEM_ROUNDING, visuals);

                                    let text_rect = rect.shrink2(egui::vec2(10.0, 0.0));
                                    painter.text(
                                        text_rect.left_center(),
                                        egui::Align2::LEFT_CENTER,
                                        &inst.name,
                                        egui::FontId::proportional(13.0),
                                        if selected {
                                            egui::Color32::WHITE
                                        } else {
                                            theme::Colors::TEXT_PRIMARY
                                        },
                                    );

                                    if let Some(loader) = &inst.mod_loader {
                                        let badge = format!("[{}]", loader.loader_type);
                                        painter.text(
                                            text_rect.right_center(),
                                            egui::Align2::RIGHT_CENTER,
                                            &badge,
                                            egui::FontId::proportional(11.0),
                                            theme::Colors::TEXT_MUTED,
                                        );
                                    }

                                    response
                                })
                                .inner;

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
