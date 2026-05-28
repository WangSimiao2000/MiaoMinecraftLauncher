use eframe::egui;

use crate::theme;

pub enum ModCardAction {
    None,
    Toggle,
    Delete,
}

pub struct ModCard<'a> {
    name: &'a str,
    enabled: bool,
}

impl<'a> ModCard<'a> {
    pub fn new(name: &'a str, enabled: bool) -> Self {
        Self { name, enabled }
    }

    pub fn show(self, ui: &mut egui::Ui) -> ModCardAction {
        let mut action = ModCardAction::None;
        let card_id = egui::Id::new(self.name).with("mod_card");

        theme::list_item_card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let card_rect = ui.min_rect();
            let card_response = ui.interact(card_rect, card_id, egui::Sense::hover());
            let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                card_id.with("hover"),
                card_response.hovered(),
                0.2,
                crate::animation::ease_linear,
            );
            if hover_t > 0.0 {
                ui.painter().rect_filled(
                    card_rect,
                    egui::CornerRadius::same(6),
                    crate::animation::lerp_color(
                        theme::Colors::bg_elevated(),
                        theme::Colors::bg_widget_hover(),
                        hover_t * 0.5,
                    ),
                );
            }

            ui.horizontal(|ui| {
                if self.enabled {
                    let btn = egui::Button::new(
                        egui::RichText::new("ON")
                            .color(theme::Colors::success())
                            .size(12.0),
                    );
                    if ui.add(btn).clicked() {
                        action = ModCardAction::Toggle;
                    }
                    ui.label(theme::body(self.name));
                } else {
                    let btn = egui::Button::new(
                        egui::RichText::new("OFF")
                            .color(theme::Colors::text_muted())
                            .size(12.0),
                    );
                    if ui.add(btn).clicked() {
                        action = ModCardAction::Toggle;
                    }
                    ui.label(egui::RichText::new(self.name).color(theme::Colors::text_disabled()));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Del").clicked() {
                        action = ModCardAction::Delete;
                    }
                });
            });
        });

        action
    }
}
