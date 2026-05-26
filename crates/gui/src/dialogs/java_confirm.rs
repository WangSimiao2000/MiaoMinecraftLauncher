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

        let anim_id = egui::Id::new("java_confirm_dialog_anim");
        let t = ctx.animate_bool_with_time(anim_id, true, 0.15);
        let opacity = t;

        let frame = egui::Frame::window(&ctx.style())
            .fill(theme::Colors::bg_elevated())
            .stroke(egui::Stroke::new(
                1.0,
                theme::Colors::accent().gamma_multiply(0.5),
            ))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::same(16.0));

        egui::Window::new("Java Required")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_opacity(opacity);
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("☕")
                            .size(32.0)
                            .color(theme::Colors::warning()),
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
                                .fill(theme::Colors::accent()),
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
