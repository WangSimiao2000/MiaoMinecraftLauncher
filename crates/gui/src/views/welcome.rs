use eframe::egui;

use crate::app::{AppView, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_welcome_setup(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(80.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::heading("Welcome to MMCL"));
                ui.add_space(theme::Spacing::SECTION_GAP);
                ui.label(theme::body("A lightweight Minecraft launcher for Linux"));
                ui.add_space(40.0);

                ui.label(theme::subheading("Data Directory"));
                ui.add_space(theme::Spacing::SMALL_GAP);
                ui.label(theme::muted(
                    "This is where instances, libraries, and assets will be stored.",
                ));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.data_dir_input)
                            .vertical_align(egui::Align::Center)
                            .min_size(ui.spacing().interact_size),
                    );
                    if ui.button("Browse").clicked()
                        && let Some(folder) = rfd::FileDialog::new()
                            .set_title("Select data directory")
                            .pick_folder()
                    {
                        self.data_dir_input = folder.display().to_string();
                    }
                });

                ui.add_space(30.0);

                let btn = egui::Button::new(
                    egui::RichText::new("Get Started")
                        .size(theme::Fonts::BUTTON)
                        .strong(),
                )
                .fill(theme::Colors::ACCENT);
                if ui.add(btn).clicked() {
                    let path = std::path::PathBuf::from(&self.data_dir_input);
                    let _ = std::fs::create_dir_all(&path);
                    self.config.data_dir = path;
                    let _ = self.config.save();
                    self.app_view = AppView::Main;
                }
            });
        });
    }
}
