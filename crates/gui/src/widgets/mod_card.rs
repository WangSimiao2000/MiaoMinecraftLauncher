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

        theme::list_item_card().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                if self.enabled {
                    let btn = egui::Button::new(
                        egui::RichText::new("ON")
                            .color(theme::Colors::SUCCESS)
                            .size(12.0),
                    );
                    if ui.add(btn).clicked() {
                        action = ModCardAction::Toggle;
                    }
                    ui.label(theme::body(self.name));
                } else {
                    let btn = egui::Button::new(
                        egui::RichText::new("OFF")
                            .color(theme::Colors::TEXT_MUTED)
                            .size(12.0),
                    );
                    if ui.add(btn).clicked() {
                        action = ModCardAction::Toggle;
                    }
                    ui.label(egui::RichText::new(self.name).color(theme::Colors::TEXT_DISABLED));
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
