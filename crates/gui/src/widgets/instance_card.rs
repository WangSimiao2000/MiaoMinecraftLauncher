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

        let hovered = response.hovered();
        let frame = theme::list_item_frame(hovered, self.selected);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, theme::LIST_ITEM_ROUNDING, frame.fill);

        let text_rect = rect.shrink2(egui::vec2(10.0, 0.0));
        painter.text(
            text_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            self.name,
            egui::FontId::proportional(13.0),
            if self.selected {
                egui::Color32::WHITE
            } else {
                theme::Colors::TEXT_PRIMARY
            },
        );

        if let Some(badge) = self.loader_badge {
            let badge_text = format!("[{}]", badge);
            painter.text(
                text_rect.right_center(),
                egui::Align2::RIGHT_CENTER,
                &badge_text,
                egui::FontId::proportional(11.0),
                theme::Colors::TEXT_MUTED,
            );
        }

        response
    }
}
