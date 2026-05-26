use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::config::DownloadMirror;

use crate::app::{I18n, Language, MiaoApp, SettingsTab};
use crate::theme::{self, ThemeColors};

impl MiaoApp {
    pub fn render_settings_page(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("settings_nav")
            .resizable(false)
            .exact_width(140.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(6.0, 0.0)))
            .show_inside(ui, |ui| {
                self.render_settings_nav(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(16.0, 0.0)))
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
                            SettingsTab::About => self.render_tab_about(ui),
                        }
                        ui.add_space(16.0);
                    });
            });
    }

    fn render_settings_nav(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.vertical(|ui| {
            ui.set_min_width(ui.available_width());
            ui.add_space(8.0);

            let tabs = [
                (SettingsTab::Account, I18n::t(lang, "account")),
                (SettingsTab::Appearance, I18n::t(lang, "appearance")),
                (SettingsTab::Data, I18n::t(lang, "data")),
                (SettingsTab::Java, I18n::t(lang, "java")),
                (SettingsTab::About, I18n::t(lang, "about")),
            ];

            for (tab, label) in tabs {
                let selected = self.settings_tab == tab;
                let text = if selected {
                    egui::RichText::new(label)
                        .strong()
                        .color(theme::Colors::accent_light())
                } else {
                    egui::RichText::new(label).color(theme::Colors::text_secondary())
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
                        theme::Colors::accent(),
                    );
                }

                if response.clicked() {
                    self.settings_tab = tab;
                }
            }
        });
    }

    fn render_tab_account(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        ui.label(theme::subheading(I18n::t(lang, "active_account")));
        ui.add_space(8.0);

        if self.config.accounts.is_empty() {
            ui.label(theme::muted(I18n::t(lang, "no_accounts")));
        } else {
            let account_info: Vec<(String, String, String, Option<String>, bool)> = self
                .config
                .accounts
                .iter()
                .enumerate()
                .map(|(i, acc)| {
                    let active = self.config.active_account_index == Some(i);
                    let avatar_url = acc.avatar_url();
                    let cape_url = acc.cape_url();
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
                    (name, kind, avatar_url, cape_url, active)
                })
                .collect();

            let mut to_delete: Option<usize> = None;
            let mut set_active: Option<usize> = None;

            for (i, (name, kind, avatar_url, cape_url, active)) in account_info.iter().enumerate() {
                egui::Frame::none()
                    .fill(if *active {
                        theme::Colors::bg_widget_hover()
                    } else {
                        theme::Colors::bg_elevated()
                    })
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                            let img = egui::Image::new(avatar_url.as_str())
                                .rounding(egui::Rounding::same(4.0));
                            img.paint_at(ui, rect);
                            if let Some(cape) = cape_url {
                                ui.add_space(4.0);
                                let (cape_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(22.0, 32.0),
                                    egui::Sense::hover(),
                                );
                                let cape_img = egui::Image::new(cape.as_str())
                                    .rounding(egui::Rounding::same(2.0));
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
                    .add(egui::SelectableLabel::new(is_classic, "Classic (Steve)"))
                    .clicked()
                {
                    self.offline_skin_model = miao_core::auth::SkinModel::Classic;
                }
                if ui
                    .add(egui::SelectableLabel::new(!is_classic, "Slim (Alex)"))
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
                        .rect_filled(rect, egui::Rounding::same(3.0), accent);

                    let text = if selected {
                        egui::RichText::new(preset.name())
                            .strong()
                            .color(theme::Colors::accent_light())
                    } else {
                        egui::RichText::new(preset.name()).color(theme::Colors::text_primary())
                    };

                    if ui.add(egui::SelectableLabel::new(selected, text)).clicked() && !selected {
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
                    .add(egui::SelectableLabel::new(selected, lang_option.name()))
                    .clicked()
                {
                    self.language = lang_option;
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
                    if ui.selectable_label(selected, text).clicked() {
                        self.language = l;
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
            ui.label(theme::muted("MiaoMinecraftLauncher v0.1.0"));
            ui.add_space(4.0);
            ui.label(theme::body("A feature-rich Minecraft launcher for Linux."));
        });

        if let Some(ref version) = self.update_available.clone() {
            ui.add_space(12.0);
            egui::Frame::none()
                .fill(theme::Colors::bg_elevated())
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::same(12.0))
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
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted("License: GPL-3.0-or-later"));
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
