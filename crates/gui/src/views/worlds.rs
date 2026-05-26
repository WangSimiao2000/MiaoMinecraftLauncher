use eframe::egui;
use std::path::Path;

use crate::app::{I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_worlds_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        self.file_scan_cache.get_or_scan(instance_dir);
        let saves = self.file_scan_cache.saves.clone();
        let saves_dir = miao_core::instance::Instance::saves_dir(instance_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!(
                "{} ({})",
                I18n::t(lang, "tab_worlds"),
                saves.len()
            )));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "open")).clicked() {
                    let _ = miao_core::instance::open_folder(&saves_dir);
                }
            });
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        if saves.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(lang, "no_worlds")));
                ui.add_space(8.0);
                ui.label(theme::small(I18n::t(lang, "no_worlds_hint")));
            });
        } else {
            for s in &saves {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(crate::icons::ICON_GLOBE)
                                .size(14.0)
                                .color(theme::Colors::accent_light()),
                        );
                        ui.label(theme::body(&s.name));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button(I18n::t(lang, "del")).clicked() {
                                let _ = s.delete();
                            }
                            if ui.small_button(I18n::t(lang, "open")).clicked() {
                                let _ = open::that(&s.path);
                            }
                            if let Ok(meta) = std::fs::metadata(&s.path)
                                && let Ok(modified) = meta.modified()
                                && let Ok(elapsed) = modified.elapsed()
                            {
                                let days = elapsed.as_secs() / 86400;
                                let label = if days == 0 {
                                    I18n::t(lang, "today").to_string()
                                } else if days == 1 {
                                    I18n::t(lang, "yesterday").to_string()
                                } else {
                                    format!("{}{}", days, I18n::t(lang, "days_ago"))
                                };
                                ui.label(theme::small(&label));
                            }
                        });
                    });
                });
                ui.add_space(2.0);
            }
        }
    }
}
