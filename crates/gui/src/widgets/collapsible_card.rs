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
            0.25,
            eframe::emath::easing::cubic_out,
        );

        let frame = egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(0))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(6)));

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let header_rect =
                egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), 36.0));
            let header_response = ui.allocate_rect(header_rect, egui::Sense::click());

            let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                self.id.with("hover"),
                header_response.hovered(),
                0.12,
                eframe::emath::easing::cubic_out,
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
                    theme::Colors::bg_widget_hover().gamma_multiply(hover_t),
                );
            }

            let arrow_rotation = openness * std::f32::consts::FRAC_PI_2;
            let arrow_center = header_rect.left_center() + egui::vec2(19.0, 0.0);
            let arrow_galley = ui.painter().layout_no_wrap(
                crate::icons::ICON_CHEVRON_RIGHT.to_string(),
                egui::FontId::proportional(10.0),
                theme::Colors::text_muted(),
            );
            ui.painter().add(egui::Shape::Text(egui::epaint::TextShape {
                pos: arrow_center - arrow_galley.rect.center().to_vec2(),
                galley: arrow_galley,
                underline: egui::Stroke::NONE,
                fallback_color: theme::Colors::text_muted(),
                override_text_color: None,
                opacity_factor: 1.0,
                angle: arrow_rotation,
            }));

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

            if openness > 0.0 {
                let inner_margin = egui::Margin::symmetric(14, 0);
                egui::Frame::NONE.inner_margin(inner_margin).show(ui, |ui| {
                    ui.set_opacity(openness);
                    ui.set_max_height(ui.available_height().max(500.0) * openness);
                    add_body(ui);
                });
                ui.add_space(14.0 * openness);
            }
        })
    }
}
