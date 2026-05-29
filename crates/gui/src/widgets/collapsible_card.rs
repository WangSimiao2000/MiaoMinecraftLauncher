use eframe::egui;

use crate::theme;

pub struct CollapsibleCard<'a> {
    title: &'a str,
    id: egui::Id,
    default_open: bool,
}

impl<'a> CollapsibleCard<'a> {
    pub fn new(title: &'a str, id: impl std::hash::Hash) -> Self {
        Self {
            title,
            id: egui::Id::new(id),
            default_open: true,
        }
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn show(
        self,
        ui: &mut egui::Ui,
        add_body: impl FnOnce(&mut egui::Ui),
    ) -> egui::InnerResponse<()> {
        let mut open = ui
            .ctx()
            .data_mut(|d| *d.get_persisted_mut_or(self.id, self.default_open));

        let openness = ui.ctx().animate_bool_with_time_and_easing(
            self.id.with("collapse"),
            open,
            0.35,
            crate::animation::ease_out,
        );

        let frame = egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(0))
            .stroke(egui::Stroke::new(1.0, theme::Colors::subtle_border()));

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let header_rect =
                egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 36.0));
            let header_response = ui.allocate_rect(header_rect, egui::Sense::click());

            let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                self.id.with("hover"),
                header_response.hovered(),
                0.2,
                crate::animation::ease_linear,
            );

            if hover_t > 0.0 {
                ui.painter().rect_filled(
                    header_rect,
                    egui::CornerRadius {
                        nw: 8,
                        ne: 8,
                        sw: if open { 0 } else { 8 },
                        se: if open { 0 } else { 8 },
                    },
                    crate::animation::lerp_color(
                        theme::Colors::bg_elevated(),
                        theme::Colors::bg_widget_hover(),
                        hover_t,
                    ),
                );
            }

            let arrow_rotation = openness * std::f32::consts::FRAC_PI_2;
            let arrow_center = header_rect.left_center() + egui::vec2(19.0, 0.0);
            let arrow_galley = ui.painter().layout_no_wrap(
                crate::icons::ICON_CHEVRON_RIGHT.to_string(),
                egui::FontId::proportional(10.0),
                theme::Colors::text_muted(),
            );
            let galley_center = arrow_galley.rect.center().to_vec2();
            let mut text_shape = egui::epaint::TextShape {
                pos: arrow_center - galley_center,
                galley: arrow_galley,
                underline: egui::Stroke::NONE,
                fallback_color: theme::Colors::text_muted(),
                override_text_color: None,
                opacity_factor: 1.0,
                angle: 0.0,
            };
            // Rotate around the visual center of the arrow glyph
            let pivot = arrow_center;
            let offset = text_shape.pos - pivot;
            let cos = arrow_rotation.cos();
            let sin = arrow_rotation.sin();
            text_shape.pos = pivot
                + egui::vec2(
                    offset.x * cos - offset.y * sin,
                    offset.x * sin + offset.y * cos,
                );
            text_shape.angle = arrow_rotation;
            ui.painter().add(egui::Shape::Text(text_shape));

            let title_pos = header_rect.left_center() + egui::vec2(32.0, 0.0);
            ui.painter().text(
                title_pos,
                egui::Align2::LEFT_CENTER,
                self.title,
                egui::FontId::proportional(theme::Fonts::SUBHEADING),
                theme::Colors::text_primary(),
            );

            if header_response.clicked() {
                open = !open;
                ui.ctx().data_mut(|d| d.insert_persisted(self.id, open));
            }

            {
                let content_id = self.id.with("content_height");
                let stored_height: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or(content_id, 0.0));

                let visible_height = (stored_height + 14.0) * openness;
                let inner_margin = egui::Margin::symmetric(14, 0);

                let where_to_put = ui.cursor();
                let clip_rect = egui::Rect::from_min_size(
                    where_to_put.min,
                    egui::vec2(ui.available_width(), visible_height),
                );
                ui.allocate_rect(clip_rect, egui::Sense::hover());

                let mut child_rect = egui::Rect::from_min_size(
                    where_to_put.min,
                    egui::vec2(ui.available_width(), stored_height.max(500.0)),
                );
                child_rect.min.x += inner_margin.left as f32;
                child_rect.max.x -= inner_margin.right as f32;

                let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(child_rect));
                child_ui.set_opacity(openness);
                child_ui.set_clip_rect(clip_rect.intersect(ui.clip_rect()));
                add_body(&mut child_ui);

                let actual_height = child_ui.min_rect().height();
                ui.ctx()
                    .data_mut(|d| d.insert_temp(content_id, actual_height));
            }
        })
    }
}
