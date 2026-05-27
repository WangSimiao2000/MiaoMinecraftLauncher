use eframe::egui;

use crate::app::{I18n, MiaoApp};
use crate::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetupStep {
    #[default]
    Language,
    Java,
    Done,
}

impl MiaoApp {
    pub fn render_setup_wizard(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();

        egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::same(24))
            .show(ui, |ui| {
                ui.set_min_width(360.0);
                ui.set_max_width(420.0);

                ui.vertical_centered(|ui| {
                    ui.label(theme::heading(I18n::t(&lang, "welcome_setup")));
                });
                ui.add_space(16.0);

                match self.setup_step {
                    SetupStep::Language => self.render_setup_language(ui),
                    SetupStep::Java => self.render_setup_java(ui),
                    SetupStep::Done => self.finish_setup(),
                }
            });
    }

    fn render_setup_language(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();
        ui.vertical_centered(|ui| {
            ui.label(theme::subheading(I18n::t(&lang, "select_language")));
        });
        ui.add_space(12.0);

        for entry in I18n::available_languages() {
            let selected = self.language == entry.id;
            let btn = if selected {
                egui::Button::new(
                    egui::RichText::new(&entry.name)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(theme::Colors::accent())
            } else {
                egui::Button::new(&entry.name)
            };
            ui.vertical_centered(|ui| {
                if ui.add_sized([200.0, 32.0], btn).clicked() {
                    self.language = entry.id.clone();
                    self.config.language = entry.id.clone();
                }
            });
            ui.add_space(4.0);
        }

        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            if ui.button(I18n::t(&self.language, "next")).clicked() {
                self.setup_step = SetupStep::Java;
                self.detect_java_for_setup();
            }
        });
    }

    fn render_setup_java(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();
        ui.vertical_centered(|ui| {
            ui.label(theme::subheading("Java"));
        });
        ui.add_space(8.0);

        if let Some(ref javas) = self.cached_javas {
            if javas.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label(theme::muted(I18n::t(&lang, "no_java_found")));
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.label(theme::body(&format!(
                        "{}: {}",
                        I18n::t(&lang, "java_found"),
                        javas.len()
                    )));
                });
                ui.add_space(4.0);
                for java in javas.iter().take(3) {
                    ui.vertical_centered(|ui| {
                        ui.label(theme::small(&format!(
                            "Java {} — {}",
                            java.version,
                            java.path.display()
                        )));
                    });
                }
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(&lang, "detecting_java")));
                ui.spinner();
            });
        }

        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            if ui.button(I18n::t(&lang, "finish")).clicked() {
                self.setup_step = SetupStep::Done;
            }
        });
    }

    fn detect_java_for_setup(&mut self) {
        let javas = miao_core::java::detect_java_with_data_dir(&self.config.data_dir);
        self.cached_javas = Some(javas);
    }

    fn finish_setup(&mut self) {
        self.config.setup_complete = true;
        let _ = self.config.save();
    }
}
