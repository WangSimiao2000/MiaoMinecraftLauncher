use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;

use crate::app::{AsyncState, MiaoApp, SettingsTab};
use crate::theme;

impl MiaoApp {
    pub fn render_settings_page(&mut self, ui: &mut egui::Ui, state: &AsyncState) {
        let nav_width = 150.0;

        ui.horizontal(|ui| {
            ui.allocate_ui(egui::vec2(nav_width, ui.available_height()), |ui| {
                self.render_settings_nav(ui);
            });
            ui.separator();
            ui.vertical(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings_content")
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        match self.settings_tab {
                            SettingsTab::Account => self.render_tab_account(ui, state),
                            SettingsTab::Data => self.render_tab_data(ui),
                            SettingsTab::Java => self.render_tab_java(ui),
                            SettingsTab::About => self.render_tab_about(ui),
                        }
                        ui.add_space(16.0);
                    });
            });
        });
    }

    fn render_settings_nav(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.set_min_width(140.0);
            ui.set_max_width(140.0);
            ui.add_space(8.0);

            let tabs = [
                (SettingsTab::Account, "Account"),
                (SettingsTab::Data, "Data"),
                (SettingsTab::Java, "Java"),
                (SettingsTab::About, "About"),
            ];

            for (tab, label) in tabs {
                let selected = self.settings_tab == tab;
                let text = if selected {
                    egui::RichText::new(label)
                        .strong()
                        .color(theme::Colors::ACCENT_LIGHT)
                } else {
                    egui::RichText::new(label).color(theme::Colors::TEXT_SECONDARY)
                };

                let response = ui.add_sized(
                    [ui.available_width(), 32.0],
                    egui::SelectableLabel::new(selected, text),
                );

                if selected {
                    let rect = response.rect;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.right() - 3.0, rect.top() + 4.0),
                            egui::pos2(rect.right(), rect.bottom() - 4.0),
                        ),
                        egui::Rounding::same(1.5),
                        theme::Colors::ACCENT,
                    );
                }

                if response.clicked() {
                    self.settings_tab = tab;
                }
            }
        });
    }

    fn render_tab_account(&mut self, ui: &mut egui::Ui, state: &AsyncState) {
        ui.label(theme::subheading("Active Account"));
        ui.add_space(8.0);

        if self.config.accounts.is_empty() {
            ui.label(theme::muted("No accounts configured yet."));
        } else {
            for (i, acc) in self.config.accounts.iter().enumerate() {
                let active = self.config.active_account_index == Some(i);
                let (prefix, acc_type) = match acc {
                    AuthMethod::Offline(a) => (a.username.as_str(), "Offline"),
                    AuthMethod::Microsoft(a) => (a.username.as_str(), "Microsoft"),
                };

                egui::Frame::none()
                    .fill(if active {
                        theme::Colors::BG_WIDGET_HOVER
                    } else {
                        theme::Colors::BG_ELEVATED
                    })
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let dot = if active { "●" } else { "○" };
                            ui.label(egui::RichText::new(dot).color(if active {
                                theme::Colors::SUCCESS
                            } else {
                                theme::Colors::TEXT_MUTED
                            }));
                            ui.label(theme::body(prefix));
                            ui.label(theme::small(acc_type));
                        });
                    });

                let rect = ui.min_rect();
                if ui
                    .interact(rect, egui::Id::new(("acc", i)), egui::Sense::click())
                    .clicked()
                {
                    self.config.active_account_index = Some(i);
                    let _ = self.config.save();
                }
                ui.add_space(4.0);
            }
        }

        ui.add_space(16.0);
        ui.label(theme::subheading("Add Account"));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::body("Offline Account"));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.offline_username_input)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text("Username"),
                );
                if ui.button("Add").clicked() && !self.offline_username_input.is_empty() {
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
        });

        ui.add_space(12.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::body("Microsoft Account"));
            ui.add_space(6.0);
            if state.auth.logging_in {
                if let Some(ref dc) = state.auth.device_code {
                    ui.label(theme::muted("Open the link below and enter the code:"));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.hyperlink_to(&dc.verification_uri, &dc.verification_uri);
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(theme::body("Code:"));
                        ui.label(
                            egui::RichText::new(&dc.user_code)
                                .strong()
                                .size(16.0)
                                .color(theme::Colors::ACCENT_LIGHT),
                        );
                    });
                    ui.add_space(4.0);
                    ui.spinner();
                } else {
                    ui.label(theme::muted("Initializing..."));
                    ui.spinner();
                }
            } else {
                ui.label(theme::muted("Sign in with your Microsoft account."));
                ui.add_space(4.0);
                if ui.button("Sign In").clicked() {
                    self.start_ms_login();
                }
            }
        });
    }

    fn render_tab_data(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::subheading("Data Directory"));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::muted(
                "Where game files are stored (instances, libraries, assets).",
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
            ui.add_space(8.0);
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

        ui.add_space(16.0);
        ui.label(theme::subheading("Downloads"));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(theme::body("Mirror:"));
                ui.label(theme::muted(&format!("{:?}", self.config.download_mirror)));
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(theme::body("Max concurrent:"));
                ui.label(theme::muted(
                    &self.config.max_concurrent_downloads.to_string(),
                ));
            });
        });
    }

    fn render_tab_java(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(theme::subheading("Java Installations"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.cached_javas = None;
                }
            });
        });
        ui.add_space(8.0);

        if self.cached_javas.is_none() {
            self.cached_javas = Some(miao_core::java::detect_system_java());
        }

        if let Some(ref javas) = self.cached_javas {
            if javas.is_empty() {
                theme::section_frame().show(ui, |ui| {
                    ui.label(theme::muted("No Java installations detected."));
                    ui.add_space(4.0);
                    ui.label(theme::small(
                        "Install Java or use 'Download Java' from an instance.",
                    ));
                });
            } else {
                for j in javas {
                    egui::Frame::none()
                        .fill(theme::Colors::BG_ELEVATED)
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("Java {}", j.major_version))
                                        .strong()
                                        .color(theme::Colors::TEXT_PRIMARY),
                                );
                                ui.label(theme::muted(&format!("({})", j.version)));
                            });
                            ui.label(theme::small(&j.path.display().to_string()));
                        });
                    ui.add_space(4.0);
                }
            }
        }
    }

    fn render_tab_about(&mut self, ui: &mut egui::Ui) {
        ui.label(theme::subheading("About MMCL"));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(
                egui::RichText::new("MMCL")
                    .size(24.0)
                    .strong()
                    .color(theme::Colors::ACCENT_LIGHT),
            );
            ui.label(theme::muted("MiaoMinecraftLauncher v0.1.0"));
            ui.add_space(4.0);
            ui.label(theme::body("A feature-rich Minecraft launcher for Linux."));
        });

        ui.add_space(16.0);
        ui.label(theme::subheading("Author"));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::body("MickeyMiao"));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(theme::muted("GitHub:"));
                ui.hyperlink_to(
                    "WangSimiao2000/MiaoMinecraftLauncher",
                    "https://github.com/WangSimiao2000/MiaoMinecraftLauncher",
                );
            });
            ui.horizontal(|ui| {
                ui.label(theme::muted("Blog:"));
                ui.hyperlink_to("blog.mickeymiao.cn", "https://blog.mickeymiao.cn");
            });
            ui.horizontal(|ui| {
                ui.label(theme::muted("Bilibili:"));
                ui.hyperlink_to("鄙人米奇喵", "https://space.bilibili.com/36913332");
            });
        });

        ui.add_space(16.0);

        theme::section_frame().show(ui, |ui| {
            ui.label(theme::muted("License: GPL-3.0-or-later"));
        });
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
