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

        let frame = egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(0))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(6)));

        frame.show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let header_response = ui
                .horizontal(|ui| {
                    ui.add_space(14.0);
                    let arrow = if open {
                        crate::icons::ICON_CHEVRON_DOWN
                    } else {
                        crate::icons::ICON_CHEVRON_RIGHT
                    };
                    ui.label(
                        egui::RichText::new(arrow)
                            .size(10.0)
                            .color(theme::Colors::text_muted()),
                    );
                    ui.label(theme::subheading(self.title));
                })
                .response;

            if header_response.interact(egui::Sense::click()).clicked() {
                open = !open;
                ui.ctx().data_mut(|d| d.insert_persisted(self.id, open));
            }

            if open {
                ui.add_space(4.0);
                let inner_margin = egui::Margin::symmetric(14, 0);
                egui::Frame::NONE.inner_margin(inner_margin).show(ui, |ui| {
                    add_body(ui);
                });
                ui.add_space(14.0);
            }
        })
    }
}
