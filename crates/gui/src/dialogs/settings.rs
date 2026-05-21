use eframe::egui;

use crate::app::{Dialog, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(theme::subheading("Configuration"));
                ui.add_space(theme::Spacing::SMALL_GAP);

                ui.horizontal(|ui| {
                    ui.label(theme::body("Data directory:"));
                    ui.label(theme::muted(&self.config.data_dir.display().to_string()));
                });
                ui.horizontal(|ui| {
                    ui.label(theme::body("Mirror:"));
                    ui.label(theme::muted(&format!("{:?}", self.config.download_mirror)));
                });
                ui.horizontal(|ui| {
                    ui.label(theme::body("Max downloads:"));
                    ui.label(theme::muted(
                        &self.config.max_concurrent_downloads.to_string(),
                    ));
                });

                ui.add_space(theme::Spacing::SECTION_GAP);
                ui.separator();
                ui.add_space(theme::Spacing::SMALL_GAP);
                ui.label(theme::subheading("Java Installations"));
                ui.add_space(theme::Spacing::SMALL_GAP);

                if self.cached_javas.is_none() {
                    self.cached_javas = Some(miao_core::java::detect_system_java());
                }
                if let Some(ref javas) = self.cached_javas {
                    if javas.is_empty() {
                        ui.label(theme::muted("None detected"));
                    } else {
                        for j in javas {
                            ui.label(theme::body(&format!(
                                "Java {} ({}) — {}",
                                j.major_version,
                                j.version,
                                j.path.display()
                            )));
                        }
                    }
                }
                ui.add_space(theme::Spacing::SMALL_GAP);
                if ui.small_button("Refresh").clicked() {
                    self.cached_javas = None;
                }
            });

        if !open {
            self.active_dialog = Dialog::None;
            self.cached_javas = None;
        }
    }
}
