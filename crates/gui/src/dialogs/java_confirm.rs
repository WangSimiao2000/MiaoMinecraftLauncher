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
        let lang = self.language.clone();
        let mut open = true;

        let anim_id = egui::Id::new("java_confirm_dialog_anim");
        let t = ctx.animate_bool_with_time(anim_id, true, 0.15);
        let opacity = t;

        let frame = egui::Frame::window(&ctx.global_style())
            .fill(theme::Colors::bg_elevated())
            .stroke(egui::Stroke::new(
                1.0,
                theme::Colors::accent().gamma_multiply(0.5),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(16));

        egui::Window::new("Java Required")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_opacity(opacity);
                ui.horizontal(|ui| {
                    ui.label(theme::subheading("Java Required"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_size = egui::vec2(28.0, 28.0);
                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());
                        let color = if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                egui::Color32::from_rgb(196, 43, 28),
                            );
                            egui::Color32::WHITE
                        } else {
                            theme::Colors::text_secondary()
                        };
                        let center = rect.center();
                        let d = 5.0;
                        let stroke = egui::Stroke::new(1.5, color);
                        ui.painter().line_segment(
                            [center - egui::vec2(d, d), center + egui::vec2(d, d)],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [center + egui::vec2(-d, d), center + egui::vec2(d, -d)],
                            stroke,
                        );
                        if response.clicked() {
                            open = false;
                        }
                    });
                });
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
                        if ui.button(I18n::t(&lang, "cancel")).clicked() {
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
