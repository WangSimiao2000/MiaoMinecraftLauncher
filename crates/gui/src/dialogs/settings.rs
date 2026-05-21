use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;

use crate::app::{AsyncState, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_settings_page(&mut self, ui: &mut egui::Ui, state: &AsyncState) {
        let available_width = ui.available_width().min(600.0);
        ui.vertical_centered(|ui| {
            ui.set_max_width(available_width);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(theme::Spacing::SECTION_GAP);

                theme::section_frame().show(ui, |ui| {
                    self.render_settings_accounts(ui, state);
                });

                ui.add_space(theme::Spacing::SECTION_GAP);

                theme::section_frame().show(ui, |ui| {
                    self.render_settings_general(ui);
                });

                ui.add_space(theme::Spacing::SECTION_GAP);

                theme::section_frame().show(ui, |ui| {
                    self.render_settings_java(ui);
                });

                ui.add_space(theme::Spacing::SECTION_GAP);

                theme::section_frame().show(ui, |ui| {
                    self.render_settings_about(ui);
                });

                ui.add_space(theme::Spacing::SECTION_GAP);
            });
        });
    }

    fn render_settings_general(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::subheading("General"));
        ui.add_space(theme::Spacing::SMALL_GAP);

        ui.horizontal(|ui| {
            ui.label(theme::body("Data directory:"));
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
            if ui.button("Apply & Migrate").clicked() {
                let new_path = std::path::PathBuf::from(&self.data_dir_input);
                let old_path = self.config.data_dir.clone();
                if new_path != old_path {
                    let _ = std::fs::create_dir_all(&new_path);
                    if old_path.exists() {
                        migrate_data_dir(&old_path, &new_path);
                    }
                    self.config.data_dir = new_path;
                    let _ = self.config.save();
                    self.instances =
                        miao_core::instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                    self.status = "Data directory migrated.".to_string();
                }
            }
        });

        ui.add_space(theme::Spacing::SMALL_GAP);
        ui.horizontal(|ui| {
            ui.label(theme::body("Download mirror:"));
            ui.label(theme::muted(&format!("{:?}", self.config.download_mirror)));
        });
        ui.horizontal(|ui| {
            ui.label(theme::body("Max concurrent downloads:"));
            ui.label(theme::muted(
                &self.config.max_concurrent_downloads.to_string(),
            ));
        });
    }

    fn render_settings_about(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::subheading("About"));
        ui.add_space(theme::Spacing::SMALL_GAP);

        ui.label(theme::body("MiaoMC Launcher v0.1.0"));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(theme::muted("GitHub:"));
            ui.hyperlink_to(
                "WangSimiao2000/MiaoMinecraftLauncher",
                "https://github.com/WangSimiao2000/MiaoMinecraftLauncher",
            );
        });
        ui.add_space(4.0);
        ui.label(theme::muted("License: GPL-3.0-or-later"));
    }

    fn render_settings_java(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(theme::subheading("Java Installations"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.cached_javas = None;
                }
            });
        });
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
    }

    fn render_settings_accounts(&mut self, ui: &mut egui::Ui, state: &AsyncState) {
        ui.label(theme::subheading("Accounts"));
        ui.add_space(theme::Spacing::SMALL_GAP);

        if self.config.accounts.is_empty() {
            ui.label(theme::muted("No accounts configured."));
        } else {
            for (i, acc) in self.config.accounts.iter().enumerate() {
                let active = self.config.active_account_index == Some(i);
                let prefix = if active { "● " } else { "○ " };
                let name = match acc {
                    AuthMethod::Offline(a) => format!("{}{} (Offline)", prefix, a.username),
                    AuthMethod::Microsoft(a) => format!("{}{} (Microsoft)", prefix, a.username),
                };
                if ui.selectable_label(active, &name).clicked() {
                    self.config.active_account_index = Some(i);
                    let _ = self.config.save();
                }
            }
        }

        ui.add_space(theme::Spacing::SECTION_GAP);
        ui.label(theme::body("Add account:"));
        ui.add_space(theme::Spacing::SMALL_GAP);

        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.add(
                egui::TextEdit::singleline(&mut self.offline_username_input)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size),
            );
            if ui.button("Add Offline").clicked() && !self.offline_username_input.is_empty() {
                let account = create_offline_account(&self.offline_username_input);
                self.config.accounts.push(AuthMethod::Offline(account));
                if self.config.active_account_index.is_none() {
                    self.config.active_account_index = Some(0);
                }
                let _ = self.config.save();
                self.offline_username_input.clear();
                self.status = "Account added.".to_string();
            }
        });

        ui.add_space(theme::Spacing::SMALL_GAP);

        if state.auth.logging_in {
            if let Some(ref dc) = state.auth.device_code {
                ui.label(theme::body(&format!(
                    "Go to: {} and enter code: {}",
                    dc.verification_uri, dc.user_code
                )));
                ui.spinner();
            } else {
                ui.label(theme::muted("Starting Microsoft login..."));
                ui.spinner();
            }
        } else if ui.button("Microsoft Login").clicked() {
            self.start_ms_login();
        }
    }
}

fn migrate_data_dir(old: &std::path::Path, new: &std::path::Path) {
    let subdirs = ["instances", "versions", "libraries", "assets", "java"];
    for dir in &subdirs {
        let src = old.join(dir);
        let dst = new.join(dir);
        if src.exists() && !dst.exists() {
            let _ = std::fs::rename(&src, &dst);
        }
    }
    let config_src = old.join("config.toml");
    let config_dst = new.join("config.toml");
    if config_src.exists() && !config_dst.exists() {
        let _ = std::fs::rename(&config_src, &config_dst);
    }
}
