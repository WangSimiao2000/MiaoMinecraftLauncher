use eframe::egui;

use crate::theme;

pub struct InstanceCard<'a> {
    name: &'a str,
    loader_badge: Option<&'a str>,
    selected: bool,
}

impl<'a> InstanceCard<'a> {
    pub fn new(name: &'a str, selected: bool) -> Self {
        Self {
            name,
            loader_badge: None,
            selected,
        }
    }

    pub fn loader(mut self, badge: Option<&'a str>) -> Self {
        self.loader_badge = badge;
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let rect = ui.available_rect_before_wrap();
        let sense = egui::Sense::click();
        let (rect, response) = ui.allocate_at_least(egui::vec2(rect.width(), 36.0), sense);

        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
            response.id.with("hover"),
            response.hovered(),
            0.12,
            eframe::emath::easing::cubic_out,
        );
        let selected_t = ui.ctx().animate_bool_with_time_and_easing(
            response.id.with("sel"),
            self.selected,
            0.15,
            eframe::emath::easing::cubic_out,
        );

        let base_color = theme::Colors::bg_elevated();
        let hover_color = theme::Colors::bg_widget_hover();
        let selected_color = theme::Colors::accent().gamma_multiply(0.3);

        let fill = if selected_t > 0.0 {
            lerp_color(base_color, selected_color, selected_t)
        } else if hover_t > 0.0 {
            hover_color.gamma_multiply(hover_t)
        } else {
            egui::Color32::TRANSPARENT
        };

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, theme::LIST_ITEM_ROUNDING, fill);

        let text_rect = rect.shrink2(egui::vec2(10.0, 0.0));
        painter.text(
            text_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            self.name,
            egui::FontId::proportional(13.0),
            if self.selected {
                theme::Colors::accent_light()
            } else {
                theme::Colors::text_primary()
            },
        );

        if let Some(badge) = self.loader_badge {
            let badge_text = format!("[{}]", badge);
            painter.text(
                text_rect.right_center(),
                egui::Align2::RIGHT_CENTER,
                &badge_text,
                egui::FontId::proportional(11.0),
                theme::Colors::text_muted(),
            );
        }

        response
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}
