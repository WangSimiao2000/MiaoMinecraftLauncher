use eframe::egui;

use crate::app::{Dialog, I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_java_download_confirm(
        &mut self,
        ctx: &egui::Context,
        instance_idx: usize,
        java_major: u32,
    ) {
        let lang = self.language;
        let mut open = true;
        egui::Window::new("Java Required")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("☕")
                            .size(32.0)
                            .color(theme::Colors::WARNING),
                    );
                    ui.add_space(8.0);
                    ui.label(theme::body(&format!(
                        "Java {} is required but not installed.",
                        java_major
                    )));
                    ui.add_space(4.0);
                    ui.label(theme::muted("Download from Adoptium? (~50-100 MB)"));
                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.add_space(60.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Download & Launch")
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(theme::Colors::ACCENT),
                            )
                            .clicked()
                        {
                            self.active_dialog = Dialog::None;
                            self.download_java_for_instance(instance_idx);
                        }
                        ui.add_space(12.0);
                        if ui.button(I18n::t(lang, "cancel")).clicked() {
                            self.active_dialog = Dialog::None;
                            self.status = "Cancelled.".to_string();
                        }
                    });
                    ui.add_space(8.0);
                });
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }
}
