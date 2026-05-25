use eframe::egui;

use crate::app::{I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_instance_settings_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;

        ui.label(theme::subheading(I18n::t(lang, "instance_settings")));
        ui.add_space(theme::Spacing::SMALL_GAP);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(theme::body(I18n::t(lang, "memory")));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(I18n::t(lang, "memory_min"));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.instance_settings_edit.memory_min)
                        .desired_width(80.0)
                        .vertical_align(egui::Align::Center),
                );
                if resp.changed() {
                    self.instance_settings_edit.dirty = true;
                }
                ui.add_space(16.0);
                ui.label(I18n::t(lang, "memory_max"));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.instance_settings_edit.memory_max)
                        .desired_width(80.0)
                        .vertical_align(egui::Align::Center),
                );
                if resp.changed() {
                    self.instance_settings_edit.dirty = true;
                }
            });
        });

        ui.add_space(theme::Spacing::SECTION_GAP);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(theme::body(I18n::t(lang, "resolution")));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(I18n::t(lang, "width"));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.instance_settings_edit.resolution_width)
                        .desired_width(80.0)
                        .vertical_align(egui::Align::Center)
                        .hint_text("1280"),
                );
                if resp.changed() {
                    self.instance_settings_edit.dirty = true;
                }
                ui.add_space(8.0);
                ui.label("×");
                ui.add_space(8.0);
                ui.label(I18n::t(lang, "height"));
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.instance_settings_edit.resolution_height)
                        .desired_width(80.0)
                        .vertical_align(egui::Align::Center)
                        .hint_text("720"),
                );
                if resp.changed() {
                    self.instance_settings_edit.dirty = true;
                }
            });
            ui.add_space(4.0);
            ui.label(theme::small("Leave empty for default resolution"));
        });

        ui.add_space(theme::Spacing::SECTION_GAP);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(theme::body(I18n::t(lang, "java_path")));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.instance_settings_edit.java_path)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text("Auto-detect"),
                );
                if resp.changed() {
                    self.instance_settings_edit.dirty = true;
                }
                if ui.button("Browse").clicked()
                    && let Some(file) = rfd::FileDialog::new()
                        .set_title("Select Java binary")
                        .pick_file()
                {
                    self.instance_settings_edit.java_path = file.display().to_string();
                    self.instance_settings_edit.dirty = true;
                }
            });
            ui.add_space(4.0);
            ui.label(theme::small("Leave empty to auto-detect compatible Java"));
        });

        ui.add_space(theme::Spacing::SECTION_GAP);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(theme::body(I18n::t(lang, "jvm_args")));
            ui.add_space(4.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.instance_settings_edit.jvm_args)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size)
                    .hint_text("-XX:+UseG1GC -XX:+ParallelRefProcEnabled"),
            );
            if resp.changed() {
                self.instance_settings_edit.dirty = true;
            }
            ui.add_space(4.0);
            ui.label(theme::small("Space-separated JVM arguments"));
        });

        ui.add_space(theme::Spacing::SECTION_GAP);

        ui.horizontal(|ui| {
            let can_save = self.instance_settings_edit.dirty;
            ui.add_enabled_ui(can_save, |ui| {
                if ui.button(I18n::t(lang, "save")).clicked() {
                    self.save_instance_settings();
                }
            });
        });
    }
}
