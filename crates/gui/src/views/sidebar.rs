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
                                if matches!(self.nav_stack.current(), Page::Settings) {
                                    self.nav_stack.pop();
                                }
                            }
                        }
                    });
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(6.0);
                    let btn_width = ui.available_width();
                    let (rect, response) =
                        ui.allocate_exact_size(egui::vec2(btn_width, 32.0), egui::Sense::click());

                    let is_active = matches!(self.nav_stack.current(), Page::Settings);

                    let bg_color = if is_active {
                        theme::Colors::bg_widget_active().gamma_multiply(0.3)
                    } else if response.hovered() {
                        theme::Colors::bg_widget_hover()
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    if bg_color != egui::Color32::TRANSPARENT {
                        ui.painter()
                            .rect_filled(rect, egui::Rounding::same(4.0), bg_color);
                    }

                    let text_color = if is_active {
                        theme::Colors::accent_light()
                    } else if response.hovered() {
                        theme::Colors::text_primary()
                    } else {
                        theme::Colors::text_secondary()
                    };

                    ui.painter().text(
                        rect.left_center() + egui::vec2(12.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("\u{2699}  {}", I18n::t(lang, "settings")),
                        egui::FontId::proportional(13.0),
                        text_color,
                    );

                    if response.clicked() {
                        if is_active {
                            self.nav_stack.pop();
                        } else {
                            self.nav_stack.push(Page::Settings);
                        }
                    }

                    ui.add_space(4.0);
                    ui.separator();
                });
            });
    }
}
