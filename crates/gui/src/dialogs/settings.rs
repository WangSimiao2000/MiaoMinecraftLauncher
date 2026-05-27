use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::config::DownloadMirror;

use crate::app::{I18n, Language, MiaoApp, SettingsTab};
use crate::theme::{self, ThemeColors};

impl MiaoApp {
    pub fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("settings_nav")
            .resizable(false)
            .exact_size(140.0)
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 0)))
            .show_inside(ui, |ui| {
                self.render_settings_nav(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(16, 0)))
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("settings_content")
                    .show(ui, |ui| {
                        ui.add_space(4.0);
                        match self.settings_tab {
                            SettingsTab::Account => self.render_tab_account(ui),
                            SettingsTab::Appearance => self.render_tab_appearance(ui),
                            SettingsTab::Data => self.render_tab_data(ui),
                            SettingsTab::Java => self.render_tab_java(ui),
                            SettingsTab::Help => self.render_tab_help(ui),
                            SettingsTab::About => self.render_tab_about(ui),
                        }
                        ui.add_space(16.0);
                    });
            });
    }

    fn render_settings_nav(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.add_space(8.0);

        let tabs = [
            (SettingsTab::Account, I18n::t(lang, "account")),
            (SettingsTab::Appearance, I18n::t(lang, "appearance")),
            (SettingsTab::Data, I18n::t(lang, "data")),
            (SettingsTab::Java, I18n::t(lang, "java")),
            (SettingsTab::Help, I18n::t(lang, "help")),
            (SettingsTab::About, I18n::t(lang, "about")),
        ];

        let panel_width = ui.available_width();
        for (tab, label) in tabs {
            let selected = self.settings_tab == tab;

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(panel_width, 32.0), egui::Sense::click());

            if selected {
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(4),
                    theme::Colors::bg_widget_active().gamma_multiply(0.3),
                );
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(
                        egui::pos2(rect.right() - 3.0, rect.top() + 4.0),
                        egui::pos2(rect.right(), rect.bottom() - 4.0),
                    ),
                    egui::CornerRadius::same(2),
                    theme::Colors::accent(),
                );
            } else if response.hovered() {
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(4),
                    theme::Colors::bg_widget_hover(),
                );
            }

            let color = if selected {
                theme::Colors::accent_light()
            } else {
                theme::Colors::text_secondary()
            };

            let font = if selected {
                egui::FontId::proportional(theme::Fonts::SUBHEADING)
            } else {
                egui::FontId::proportional(theme::Fonts::BODY)
            };

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                font,
                color,
            );

            if response.clicked() {
                self.settings_tab = tab;
            }
        }
    }

    fn render_tab_account(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.label(theme::subheading(I18n::t(lang, "active_account")));
        ui.add_space(8.0);

        if self.config.accounts.is_empty() {
            ui.label(theme::muted(I18n::t(lang, "no_accounts")));
        } else {
            let data_dir = self.config.data_dir.clone();
            let account_info: Vec<(
                String,
                String,
                std::path::PathBuf,
                Option<std::path::PathBuf>,
                bool,
            )> = self
                .config
                .accounts
                .iter()
                .enumerate()
                .map(|(i, acc)| {
                    let active = self.config.active_account_index == Some(i);
                    let avatar_path = acc.avatar_path(&data_dir);
                    let cape_path = acc.cape_path(&data_dir);
                    let (name, kind) = match acc {
                        AuthMethod::Offline(a) => (
                            a.username.clone(),
                            format!("Offline · {}", a.skin_model.display_name()),
                        ),
                        AuthMethod::Microsoft(a) => (a.username.clone(), "Microsoft".to_string()),
                        AuthMethod::AuthlibInjector(a) => {
                            (a.username.clone(), a.server_name.clone())
                        }
                    };
                    (name, kind, avatar_path, cape_path, active)
                })
                .collect();

            let mut to_delete: Option<usize> = None;
            let mut set_active: Option<usize> = None;

            for (i, (name, kind, avatar_path, cape_path, active)) in account_info.iter().enumerate()
            {
                egui::Frame::NONE
                    .fill(if *active {
                        theme::Colors::bg_widget_hover()
                    } else {
                        theme::Colors::bg_elevated()
                    })
                    .corner_radius(egui::CornerRadius::same(6))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                            let avatar_uri = format!("file://{}", avatar_path.display());
                            let img = egui::Image::new(&avatar_uri)
                                .corner_radius(egui::CornerRadius::same(4));
                            img.paint_at(ui, rect);
                            if let Some(cape) = cape_path {
                                ui.add_space(4.0);
                                let (cape_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(22.0, 32.0),
                                    egui::Sense::hover(),
                                );
                                let cape_uri = format!("file://{}", cape.display());
                                let cape_img = egui::Image::new(&cape_uri)
                                    .corner_radius(egui::CornerRadius::same(2));
                                cape_img.paint_at(ui, cape_rect);
                            }
                            ui.add_space(6.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    if *active {
                                        ui.label(
                                            egui::RichText::new(crate::icons::ICON_CIRCLE_FILL)
                                                .size(8.0)
                                                .color(theme::Colors::success()),
                                        );
                                    }
                                    ui.label(theme::body(name));
                                });
                                ui.label(theme::small(kind));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(egui::Button::new(
                                            egui::RichText::new(crate::icons::ICON_X)
                                                .size(12.0)
                                                .color(theme::Colors::danger()),
                                        ))
                                        .clicked()
                                    {
                                        to_delete = Some(i);
                                    }
                                    if !*active
                                        && ui.small_button(I18n::t(lang, "set_active")).clicked()
                                    {
                                        set_active = Some(i);
                                    }
                                },
                            );
                        });
                    });
                ui.add_space(4.0);
            }

            if let Some(idx) = set_active {
                self.config.active_account_index = Some(idx);
                if let Err(e) = self.config.save() {
                    self.status = format!("✗ Save failed: {}", e);
                }
            }

            if let Some(idx) = to_delete {
                self.config.accounts.remove(idx);
                if let Some(active) = self.config.active_account_index {
                    if active == idx {
                        self.config.active_account_index = if self.config.accounts.is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                    } else if active > idx {
                        self.config.active_account_index = Some(active - 1);
                    }
                }
                if let Err(e) = self.config.save() {
                    self.status = format!("✗ Save failed: {}", e);
                } else {
                    self.status = I18n::t(lang, "account_removed").to_string();
                }
            }
        }

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "add_account")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(lang, "offline_account")));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.offline_username_input)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text(I18n::t(lang, "username")),
                );
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(theme::small(I18n::t(lang, "skin_model")));
                ui.add_space(4.0);
                let is_classic = self.offline_skin_model == miao_core::auth::SkinModel::Classic;
                if ui
                    .add(egui::Button::new("Classic (Steve)").selected(is_classic))
                    .clicked()
                {
                    self.offline_skin_model = miao_core::auth::SkinModel::Classic;
                }
                if ui
                    .add(egui::Button::new("Slim (Alex)").selected(!is_classic))
                    .clicked()
                {
                    self.offline_skin_model = miao_core::auth::SkinModel::Slim;
                }
            });
            ui.add_space(6.0);
            if ui.button(I18n::t(lang, "add")).clicked() && !self.offline_username_input.is_empty()
            {
                let account = miao_core::auth::offline::create_offline_account_with_model(
                    &self.offline_username_input,
                    self.offline_skin_model,
                );
                self.config.accounts.push(AuthMethod::Offline(account));
                if self.config.active_account_index.is_none() {
                    self.config.active_account_index = Some(0);
                }
                if let Err(e) = self.config.save() {
                    self.status = format!("✗ Save failed: {}", e);
                } else {
                    self.offline_username_input.clear();
                    self.status = I18n::t(lang, "account_added").to_string();
                }
            }
        });

        ui.add_space(12.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(lang, "ms_account")));
            ui.add_space(6.0);
            if self.auth.logging_in {
                if let Some(ref dc) = self.auth.device_code {
                    ui.label(theme::muted(I18n::t(lang, "ms_login_hint")));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.hyperlink_to(&dc.verification_uri, &dc.verification_uri);
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(theme::body(I18n::t(lang, "ms_code")));
                        ui.label(
                            egui::RichText::new(&dc.user_code)
                                .strong()
                                .size(16.0)
                                .color(theme::Colors::accent_light()),
                        );
                    });
                    ui.add_space(4.0);
                    ui.spinner();
                } else {
                    ui.label(theme::muted(I18n::t(lang, "ms_initializing")));
                    ui.spinner();
                }
            } else {
                ui.label(theme::muted(I18n::t(lang, "ms_signin_desc")));
                ui.add_space(4.0);
                if ui.button(I18n::t(lang, "sign_in")).clicked() {
                    self.start_ms_login();
                }
            }
        });

        ui.add_space(12.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(lang, "authlib_account")));
            ui.add_space(4.0);
            ui.label(theme::muted(I18n::t(lang, "authlib_desc")));
            ui.add_space(6.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.auth.authlib_server_url)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size)
                    .hint_text("https://littleskin.cn/api/yggdrasil"),
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.auth.authlib_email)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text(I18n::t(lang, "email")),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.auth.authlib_password)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text(I18n::t(lang, "password"))
                        .password(true),
                );
            });
            ui.add_space(6.0);
            if self.auth.authlib_logging_in {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(theme::muted(I18n::t(lang, "logging_in")));
                });
            } else if ui.button(I18n::t(lang, "sign_in")).clicked()
                && !self.auth.authlib_server_url.is_empty()
                && !self.auth.authlib_email.is_empty()
                && !self.auth.authlib_password.is_empty()
            {
                self.auth.authlib_logging_in = true;
                self.controller
                    .send(crate::messages::AppCommand::StartAuthlibLogin {
                        server_url: self.auth.authlib_server_url.clone(),
                        email: self.auth.authlib_email.clone(),
                        password: self.auth.authlib_password.clone(),
                    });
            }
        });
    }

    fn render_tab_appearance(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;

        ui.label(theme::subheading(I18n::t(lang, "theme")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(lang, "theme_desc")));
            ui.add_space(12.0);

            let current = self.theme_preset;
            for preset in theme::ThemePreset::ALL {
                let selected = current == preset;
                let accent = preset.accent();

                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, egui::CornerRadius::same(3), accent);

                    let text = if selected {
                        egui::RichText::new(preset.name())
                            .strong()
                            .color(theme::Colors::accent_light())
                    } else {
                        egui::RichText::new(preset.name()).color(theme::Colors::text_primary())
                    };

                    if ui.add(egui::Button::new(text).selected(selected)).clicked() && !selected {
                        self.theme_preset = preset;
                        self.config.theme = preset;
                        if let Err(e) = self.config.save() {
                            self.status = format!("✗ Save failed: {}", e);
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "language")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            for lang_option in Language::ALL {
                let selected = self.language == lang_option;
                if ui
                    .add(egui::Button::new(lang_option.name()).selected(selected))
                    .clicked()
                    && !selected
                {
                    self.language = lang_option;
                    self.config.language = lang_option.into();
                    if let Err(e) = self.config.save() {
                        self.status = format!("✗ Save failed: {}", e);
                    }
                }
            }
        });
    }

    fn render_tab_data(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;

        ui.label(theme::subheading(I18n::t(lang, "data_dir")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(lang, "data_dir_desc")));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.data_dir_input)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size),
                );
                if ui.button(I18n::t(lang, "browse")).clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select data directory")
                        .pick_folder()
                {
                    self.data_dir_input = folder.display().to_string();
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(I18n::t(lang, "apply")).clicked() {
                    let new_path = std::path::PathBuf::from(&self.data_dir_input);
                    if let Err(e) = std::fs::create_dir_all(&new_path) {
                        self.status = format!("✗ Failed to create directory: {}", e);
                    } else {
                        self.config.data_dir = new_path;
                        if let Err(e) = self.config.save() {
                            self.status = format!("✗ Failed to save config: {}", e);
                        } else {
                            self.instances =
                                miao_core::instance::list_instances(&self.config.instances_dir())
                                    .unwrap_or_default();
                            self.selected_instance = None;
                            self.status = I18n::t(lang, "data_dir_updated").to_string();
                        }
                    }
                }
                if ui.button(I18n::t(lang, "apply_migrate")).clicked() {
                    let new_path = std::path::PathBuf::from(&self.data_dir_input);
                    let old_path = self.config.data_dir.clone();
                    if new_path != old_path {
                        if let Err(e) = std::fs::create_dir_all(&new_path) {
                            self.status = format!("✗ Failed to create directory: {}", e);
                        } else {
                            let migration_ok = if old_path.exists() {
                                migrate_data_dir(&old_path, &new_path).is_ok()
                            } else {
                                true
                            };
                            self.config.data_dir = new_path;
                            if let Err(e) = self.config.save() {
                                self.status = format!("✗ Failed to save config: {}", e);
                            } else if migration_ok {
                                self.instances = miao_core::instance::list_instances(
                                    &self.config.instances_dir(),
                                )
                                .unwrap_or_default();
                                self.selected_instance = None;
                                self.status = I18n::t(lang, "data_dir_migrated").to_string();
                            } else {
                                self.instances = miao_core::instance::list_instances(
                                    &self.config.instances_dir(),
                                )
                                .unwrap_or_default();
                                self.selected_instance = None;
                                self.status = I18n::t(lang, "data_dir_partial").to_string();
                            }
                        }
                    }
                }
            });
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "game_folders")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(lang, "game_folders_desc")));
            ui.add_space(8.0);

            let active = self.config.data_dir.display().to_string();
            let mut switch_to: Option<std::path::PathBuf> = None;
            let mut remove_idx: Option<usize> = None;

            for (i, folder) in self.config.game_folders.iter().enumerate() {
                let is_active = *folder == self.config.data_dir;
                ui.horizontal(|ui| {
                    if is_active {
                        ui.label(theme::body(&format!("▶ {}", folder.display())));
                    } else {
                        ui.label(theme::muted(&folder.display().to_string()));
                        if ui.small_button(I18n::t(lang, "switch")).clicked() {
                            switch_to = Some(folder.clone());
                        }
                    }
                    if !is_active && ui.small_button("✕").clicked() {
                        remove_idx = Some(i);
                    }
                });
            }

            if self.config.game_folders.is_empty()
                || !self.config.game_folders.contains(&self.config.data_dir)
            {
                ui.horizontal(|ui| {
                    ui.label(theme::body(&format!("▶ {}", active)));
                });
            }

            ui.add_space(8.0);
            if ui.button(I18n::t(lang, "add_folder")).clicked()
                && let Some(folder) = rfd::FileDialog::new()
                    .set_title("Select game folder")
                    .pick_folder()
                && !self.config.game_folders.contains(&folder)
            {
                self.config.game_folders.push(folder);
                let _ = self.config.save();
            }

            if let Some(idx) = remove_idx {
                self.config.game_folders.remove(idx);
                let _ = self.config.save();
            }

            if let Some(path) = switch_to {
                self.config.data_dir = path;
                self.data_dir_input = self.config.data_dir.display().to_string();
                let _ = self.config.save();
                self.instances = miao_core::instance::list_instances(&self.config.instances_dir())
                    .unwrap_or_default();
                self.selected_instance = None;
                self.cached_javas = None;
                self.status = I18n::t(lang, "folder_switched").to_string();
            }
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "mirror")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let current_mirror = match &self.config.download_mirror {
                DownloadMirror::Official => 0,
                DownloadMirror::Bmclapi => 1,
                DownloadMirror::Custom(_) => 2,
            };
            let mut selected = current_mirror;

            ui.horizontal(|ui| {
                ui.radio_value(&mut selected, 0, I18n::t(lang, "mirror_official"));
                ui.radio_value(&mut selected, 1, I18n::t(lang, "mirror_bmclapi"));
                ui.radio_value(&mut selected, 2, I18n::t(lang, "mirror_custom"));
            });

            if selected == 2 {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.mirror_custom_url)
                            .vertical_align(egui::Align::Center)
                            .min_size(ui.spacing().interact_size)
                            .hint_text("https://my-mirror.example.com"),
                    );
                });
            }

            if selected != current_mirror {
                let new_mirror = match selected {
                    0 => DownloadMirror::Official,
                    1 => DownloadMirror::Bmclapi,
                    _ => DownloadMirror::Custom(self.mirror_custom_url.clone()),
                };
                self.config.download_mirror = new_mirror;
                if let Err(e) = self.config.save() {
                    self.status = format!("✗ Save failed: {}", e);
                } else {
                    self.status = I18n::t(lang, "mirror_updated").to_string();
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.label(theme::body(I18n::t(lang, "max_concurrent")));
                ui.add_space(8.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.max_downloads_input)
                        .desired_width(60.0)
                        .vertical_align(egui::Align::Center),
                );
                if resp.lost_focus() {
                    if let Ok(v) = self.max_downloads_input.parse::<usize>() {
                        let v = v.clamp(1, 256);
                        self.config.max_concurrent_downloads = v;
                        self.max_downloads_input = v.to_string();
                        if let Err(e) = self.config.save() {
                            self.status = format!("✗ Save failed: {}", e);
                        }
                    } else {
                        self.max_downloads_input = self.config.max_concurrent_downloads.to_string();
                    }
                }
            });
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "cf_api_key")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(lang, "cf_api_key_desc")));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.cf_api_key_input)
                        .password(true)
                        .vertical_align(egui::Align::Center)
                        .min_size(ui.spacing().interact_size)
                        .hint_text("$2a$10$..."),
                );
                if resp.lost_focus() {
                    let key = if self.cf_api_key_input.trim().is_empty() {
                        None
                    } else {
                        Some(self.cf_api_key_input.trim().to_string())
                    };
                    self.config.curseforge_api_key = key;
                    if let Err(e) = self.config.save() {
                        self.status = format!("✗ Save failed: {}", e);
                    }
                }
            });
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "language")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                for l in Language::ALL {
                    let selected = self.language == l;
                    let text = if selected {
                        egui::RichText::new(l.name())
                            .strong()
                            .color(theme::Colors::accent_light())
                    } else {
                        egui::RichText::new(l.name()).color(theme::Colors::text_secondary())
                    };
                    if ui.selectable_label(selected, text).clicked() && !selected {
                        self.language = l;
                        self.config.language = l.into();
                        if let Err(e) = self.config.save() {
                            self.status = format!("✗ Save failed: {}", e);
                        }
                    }
                }
            });
        });
    }

    fn render_tab_java(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(lang, "java_installs")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "refresh")).clicked() {
                    self.cached_javas = None;
                }
            });
        });
        ui.add_space(8.0);

        if self.cached_javas.is_none() {
            self.cached_javas = Some(miao_core::java::detect_java_with_data_dir(
                &self.config.data_dir,
            ));
        }

        if let Some(ref javas) = self.cached_javas {
            if javas.is_empty() {
                theme::section_frame().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(theme::muted("No Java installations detected."));
                    ui.add_space(4.0);
                    ui.label(theme::small(
                        "Install Java or use 'Download Java' from an instance.",
                    ));
                });
            } else {
                for j in javas {
                    theme::list_item_card().show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Java {}", j.major_version))
                                    .strong()
                                    .color(theme::Colors::text_primary()),
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

    fn render_tab_help(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;

        ui.label(theme::subheading(I18n::t(lang, "faq")));
        ui.add_space(8.0);

        let faqs: &[(&str, &str)] = match lang {
            Language::English => &[
                (
                    "Game won't launch / crashes immediately",
                    "Check that you have a compatible Java version installed (Settings > Java). \
                     Minecraft 1.17+ requires Java 17+, and 1.20.5+ requires Java 21+. \
                     MMCL can auto-download the correct version if needed.",
                ),
                (
                    "Mods not showing up after install",
                    "Ensure the mod is compatible with both your Minecraft version and mod loader. \
                     Fabric mods won't work with Forge/NeoForge and vice versa.",
                ),
                (
                    "Microsoft login fails or times out",
                    "Make sure you complete the device code login within 15 minutes. \
                     If it keeps failing, check your network connection and try again.",
                ),
                (
                    "Downloads are slow",
                    "Try switching to BMCLAPI mirror in Settings > Data. \
                     It significantly speeds up downloads in mainland China.",
                ),
                (
                    "How to use a third-party skin site (LittleSkin)?",
                    "Add an authlib-injector account in Settings > Account. \
                     Enter the skin server URL (e.g. https://littleskin.cn/api/yggdrasil).",
                ),
                (
                    "How to import/export modpacks?",
                    "Use the Export button on an instance to create a .mrpack file. \
                     Use the Import button in the sidebar to load one. \
                     Only Modrinth .mrpack format is supported.",
                ),
            ],
            Language::Chinese => &[
                (
                    "游戏无法启动 / 立即崩溃",
                    "检查是否安装了兼容的 Java 版本（设置 > Java）。\
                     Minecraft 1.17+ 需要 Java 17+，1.20.5+ 需要 Java 21+。\
                     MMCL 可以自动下载正确版本。",
                ),
                (
                    "安装 Mod 后不显示",
                    "确保 Mod 与你的 Minecraft 版本和加载器兼容。\
                     Fabric Mod 不能用于 Forge/NeoForge，反之亦然。",
                ),
                (
                    "微软登录失败或超时",
                    "请在 15 分钟内完成设备码登录。\
                     若持续失败，检查网络连接后重试。",
                ),
                (
                    "下载速度很慢",
                    "尝试在 设置 > 数据 中切换到 BMCLAPI 镜像源，\
                     可显著提高国内下载速度。",
                ),
                (
                    "如何使用第三方皮肤站（LittleSkin）？",
                    "在 设置 > 账户 中添加 authlib-injector 账号，\
                     输入皮肤站地址（如 https://littleskin.cn/api/yggdrasil）。",
                ),
                (
                    "如何导入/导出整合包？",
                    "点击实例的「导出」按钮生成 .mrpack 文件。\
                     点击侧边栏的「导入」按钮加载整合包。\
                     目前仅支持 Modrinth .mrpack 格式。",
                ),
            ],
        };

        for (question, answer) in faqs {
            crate::widgets::CollapsibleCard::new(question, question)
                .default_open(false)
                .show(ui, |ui| {
                    ui.label(theme::body(answer));
                });
            ui.add_space(4.0);
        }
    }

    fn render_tab_about(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.label(theme::subheading(I18n::t(lang, "about")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("MMCL")
                    .size(24.0)
                    .strong()
                    .color(theme::Colors::accent_light()),
            );
            ui.label(theme::muted(&format!(
                "MiaoMinecraftLauncher v{}",
                env!("CARGO_PKG_VERSION")
            )));
            ui.add_space(4.0);
            ui.label(theme::body(
                "A feature-rich cross-platform Minecraft launcher.",
            ));
        });

        if let Some(ref version) = self.update_available.clone() {
            ui.add_space(12.0);
            egui::Frame::NONE
                .fill(theme::Colors::bg_elevated())
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::same(12))
                .stroke(egui::Stroke::new(1.5_f32, theme::Colors::warning()))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("⬆")
                                .size(16.0)
                                .color(theme::Colors::warning()),
                        );
                        ui.label(theme::body(&format!(
                            "{}: {}",
                            I18n::t(self.language, "update_available"),
                            version
                        )));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button(I18n::t(lang, "download")).clicked() {
                                    let _ = open::that(
                                        "https://github.com/WangSimiao2000/MiaoMinecraftLauncher/releases",
                                    );
                                }
                            },
                        );
                    });
                });
        }

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "author")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
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
                ui.label(theme::muted("Bilibili:"));
                ui.hyperlink_to("鄙人米奇喵", "https://space.bilibili.com/36913332");
            });
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(lang, "open_source_credits")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted("License: GPL-3.0-or-later"));
            ui.add_space(8.0);
            ui.label(theme::muted("Fonts:"));
            ui.horizontal(|ui| {
                ui.label(theme::muted("  ·"));
                ui.hyperlink_to(
                    "MiSans — Xiaomi (SIL OFL 1.1)",
                    "https://hyperos.mi.com/font/en",
                );
            });
            ui.horizontal(|ui| {
                ui.label(theme::muted("  ·"));
                ui.hyperlink_to(
                    "Bootstrap Icons — The Bootstrap Authors (MIT)",
                    "https://icons.getbootstrap.com/",
                );
            });
        });
    }
}

fn migrate_data_dir(old: &std::path::Path, new: &std::path::Path) -> std::io::Result<()> {
    let subdirs = ["instances", "versions", "libraries", "assets", "java"];
    for dir in &subdirs {
        let src = old.join(dir);
        let dst = new.join(dir);
        if src.exists() && !dst.exists() {
            std::fs::rename(&src, &dst)?;
        }
    }
    let config_src = old.join("config.toml");
    let config_dst = new.join("config.toml");
    if config_src.exists() && !config_dst.exists() {
        std::fs::rename(&config_src, &config_dst)?;
    }
    Ok(())
}
