use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::config::{DownloadMirror, JavaSource};

use crate::app::{I18n, MiaoApp, SettingsTab};
use crate::theme;

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
                let content_t = ui.ctx().animate_bool_with_time_and_easing(
                    egui::Id::new("settings_content_fade").with(self.settings_tab as u8),
                    true,
                    0.3,
                    crate::animation::ease_linear,
                );
                ui.set_opacity(content_t);

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
        let lang = self.language.clone();
        ui.add_space(8.0);

        let tabs = [
            (SettingsTab::Account, I18n::t(&lang, "account")),
            (SettingsTab::Appearance, I18n::t(&lang, "appearance")),
            (SettingsTab::Data, I18n::t(&lang, "data")),
            (SettingsTab::Java, I18n::t(&lang, "java")),
            (SettingsTab::Help, I18n::t(&lang, "help")),
            (SettingsTab::About, I18n::t(&lang, "about")),
        ];

        let panel_width = ui.available_width();
        let nav_top = ui.cursor().top();

        for (tab, label) in tabs {
            let selected = self.settings_tab == tab;

            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(panel_width, 32.0), egui::Sense::click());

            let tab_id = egui::Id::new(label).with("settings_nav");
            let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                tab_id.with("sel"),
                selected,
                0.3,
                crate::animation::ease_linear,
            );
            let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                tab_id.with("hover"),
                response.hovered() && !selected,
                0.2,
                crate::animation::ease_linear,
            );

            if sel_t > 0.0 {
                let c = theme::Colors::bg_widget_active();
                let alpha = (0.3 * sel_t * 255.0) as u8;
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(4),
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                );
            }
            if hover_t > 0.0 {
                let c = theme::Colors::bg_widget_hover();
                let alpha = (hover_t * 255.0) as u8;
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::same(4),
                    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha),
                );
            }

            if selected {
                let target_y = rect.center().y - nav_top;
                if self.settings_nav_indicator_y.position() == 0.0
                    && self.settings_nav_indicator_y.velocity() == 0.0
                {
                    self.settings_nav_indicator_y.snap(target_y);
                } else {
                    self.settings_nav_indicator_y.set_target(target_y);
                }
            }

            let active_color = theme::Colors::accent_light();
            let idle_color = theme::Colors::text_secondary();
            let color = crate::animation::lerp_color(idle_color, active_color, sel_t);

            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(theme::Fonts::BODY),
                color,
            );

            if response.clicked() {
                self.settings_tab = tab;
            }
        }

        let indicator_y = nav_top + self.settings_nav_indicator_y.position();
        let panel_right = ui.min_rect().right();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(panel_right - 3.0, indicator_y - 12.0),
                egui::vec2(3.0, 24.0),
            ),
            egui::CornerRadius::same(2),
            theme::Colors::accent(),
        );
    }

    fn render_tab_account(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();
        ui.label(theme::subheading(I18n::t(&lang, "active_account")));
        ui.add_space(8.0);

        if self.config.accounts.is_empty() {
            theme::section_frame().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(theme::muted(I18n::t(&lang, "no_accounts")));
            });
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
                let card_id = egui::Id::new("account_card").with(i);
                let active_t = ui.ctx().animate_bool_with_time_and_easing(
                    card_id.with("active"),
                    *active,
                    0.3,
                    crate::animation::ease_linear,
                );
                let base = theme::Colors::bg_elevated();
                let active_color = theme::Colors::bg_widget_hover();
                let fill = crate::animation::lerp_color(base, active_color, active_t);

                egui::Frame::NONE
                    .fill(fill)
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
                                        && ui.small_button(I18n::t(&lang, "set_active")).clicked()
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
                    self.status = I18n::t(&lang, "account_removed").to_string();
                }
            }
        }

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(&lang, "add_account")));
        ui.add_space(8.0);

        // ── Offline Account ─────────────────────────────────────────────
        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(&lang, "offline_account")));
            ui.add_space(8.0);

            ui.label(theme::small(I18n::t(&lang, "username")));
            ui.add_space(3.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.offline_username_input)
                    .vertical_align(egui::Align::Center)
                    .desired_width(ui.available_width())
                    .hint_text(I18n::t(&lang, "username")),
            );

            ui.add_space(8.0);
            ui.label(theme::small(I18n::t(&lang, "skin_model")));
            ui.add_space(3.0);

            let cols = 2;
            let total_width = ui.available_width();
            let card_width = (total_width - 8.0 * (cols - 1) as f32) / cols as f32;
            let skin_labels = ["Classic (Steve)", "Slim (Alex)"];
            let is_classic = self.offline_skin_model == miao_core::auth::SkinModel::Classic;
            let skin_selected = [is_classic, !is_classic];

            egui::Grid::new("skin_model_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 0.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    for (i, label) in skin_labels.iter().enumerate() {
                        let selected = skin_selected[i];
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 32.0),
                            egui::Sense::click(),
                        );

                        let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("skin_card").with(i).with("sel"),
                            selected,
                            0.25,
                            crate::animation::ease_linear,
                        );
                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("skin_card").with(i).with("hover"),
                            response.hovered(),
                            0.2,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = if sel_t > 0.0 {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::accent(),
                                0.15 * sel_t,
                            )
                        } else {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::bg_widget_hover(),
                                hover_t,
                            )
                        };

                        let stroke = if sel_t > 0.0 {
                            egui::Stroke::new(1.5, theme::Colors::accent().gamma_multiply(sel_t))
                        } else {
                            egui::Stroke::new(1.0, theme::Colors::subtle_border())
                        };

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let text_color = if selected {
                            theme::Colors::accent_light()
                        } else {
                            theme::Colors::text_primary()
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            *label,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        if response.clicked() && !selected {
                            self.offline_skin_model = if i == 0 {
                                miao_core::auth::SkinModel::Classic
                            } else {
                                miao_core::auth::SkinModel::Slim
                            };
                        }
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(I18n::t(&lang, "add")).clicked()
                        && !self.offline_username_input.is_empty()
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
                            self.status = I18n::t(&lang, "account_added").to_string();
                        }
                    }
                });
            });
        });

        ui.add_space(12.0);

        // ── Microsoft Account ───────────────────────────────────────────
        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(&lang, "ms_account")));
            ui.add_space(8.0);
            if self.auth.logging_in {
                if let Some(ref dc) = self.auth.device_code {
                    ui.label(theme::muted(I18n::t(&lang, "ms_login_hint")));
                    ui.add_space(8.0);
                    ui.hyperlink_to(&dc.verification_uri, &dc.verification_uri);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(theme::body(I18n::t(&lang, "ms_code")));
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(&dc.user_code)
                                .strong()
                                .size(16.0)
                                .color(theme::Colors::accent_light()),
                        );
                    });
                    ui.add_space(8.0);
                    ui.spinner();
                } else {
                    ui.label(theme::muted(I18n::t(&lang, "ms_initializing")));
                    ui.add_space(4.0);
                    ui.spinner();
                }
            } else {
                ui.label(theme::muted(I18n::t(&lang, "ms_signin_desc")));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(I18n::t(&lang, "sign_in")).clicked() {
                            self.start_ms_login();
                        }
                    });
                });
            }
        });

        ui.add_space(12.0);

        // ── Authlib-Injector Account ────────────────────────────────────
        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::body(I18n::t(&lang, "authlib_account")));
            ui.add_space(4.0);
            ui.label(theme::muted(I18n::t(&lang, "authlib_desc")));
            ui.add_space(12.0);

            ui.label(theme::small("Server URL"));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.auth.authlib_server_url)
                    .vertical_align(egui::Align::Center)
                    .desired_width(ui.available_width())
                    .hint_text("https://littleskin.cn/api/yggdrasil"),
            );

            ui.add_space(10.0);
            ui.label(theme::small(I18n::t(&lang, "email")));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.auth.authlib_email)
                    .vertical_align(egui::Align::Center)
                    .desired_width(ui.available_width())
                    .hint_text(I18n::t(&lang, "email")),
            );

            ui.add_space(10.0);
            ui.label(theme::small(I18n::t(&lang, "password")));
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.auth.authlib_password)
                    .vertical_align(egui::Align::Center)
                    .desired_width(ui.available_width())
                    .hint_text(I18n::t(&lang, "password"))
                    .password(true),
            );

            ui.add_space(12.0);
            if self.auth.authlib_logging_in {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(theme::muted(I18n::t(&lang, "logging_in")));
                });
            } else {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(I18n::t(&lang, "sign_in")).clicked()
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
                });
            }
        });
    }

    fn render_tab_appearance(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();

        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(&lang, "theme")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(I18n::t(&lang, "refresh")).clicked() {
                    self.custom_themes =
                        miao_core::custom_theme::load_themes_from_dir(&self.config.themes_dir());
                    if self.active_custom_theme.is_some()
                        && !self
                            .custom_themes
                            .iter()
                            .any(|t| Some(&t.file_name) == self.active_custom_theme.as_ref())
                    {
                        self.active_custom_theme = None;
                        self.config.custom_theme_name = None;
                        let _ = self.config.save();
                    }
                }
            });
        });
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(&lang, "theme_desc")));
            ui.add_space(12.0);

            let current_preset = self.theme_preset;
            let custom_themes = self.custom_themes.clone();
            let total_count = theme::ThemePreset::ALL.len() + custom_themes.len();
            let cols = 3;
            let total_width = ui.available_width();
            let card_width = (total_width - 8.0 * (cols - 1) as f32) / cols as f32;

            egui::Grid::new("theme_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 8.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    for idx in 0..total_count {
                        let is_builtin = idx < theme::ThemePreset::ALL.len();

                        let (swatch_bg, swatch_accent, label, selected) = if is_builtin {
                            let preset = theme::ThemePreset::ALL[idx];
                            let (bg, acc) = theme::swatch_colors(preset);
                            let sel =
                                self.active_custom_theme.is_none() && current_preset == preset;
                            (bg, acc, I18n::t(&lang, preset.i18n_key()).to_string(), sel)
                        } else {
                            let ct = &custom_themes[idx - theme::ThemePreset::ALL.len()];
                            let (bg, acc) = theme::custom_swatch_colors(&ct.theme.colors);
                            let sel = self.active_custom_theme.as_deref() == Some(&ct.file_name);
                            let name = ct.theme.name.clone();
                            (bg, acc, name, sel)
                        };

                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 56.0),
                            egui::Sense::click(),
                        );

                        let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("theme_card").with(idx).with("sel"),
                            selected,
                            crate::animation::DURATION_SELECT,
                            crate::animation::ease_linear,
                        );
                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("theme_card").with(idx).with("hover"),
                            response.hovered(),
                            crate::animation::DURATION_HOVER,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = if sel_t > 0.0 {
                            crate::animation::lerp_color(base, swatch_accent, 0.15 * sel_t)
                        } else {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::bg_widget_hover(),
                                hover_t,
                            )
                        };

                        let stroke = if sel_t > 0.0 {
                            egui::Stroke::new(1.5, swatch_accent.gamma_multiply(sel_t))
                        } else {
                            egui::Stroke::new(1.0, theme::Colors::subtle_border())
                        };

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let swatch_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.center().x - 12.0, rect.top() + 8.0),
                            egui::vec2(24.0, 24.0),
                        );
                        ui.painter().rect_filled(
                            swatch_rect,
                            egui::CornerRadius::same(5),
                            swatch_bg,
                        );
                        ui.painter().rect_stroke(
                            swatch_rect,
                            egui::CornerRadius::same(5),
                            egui::Stroke::new(1.0, swatch_accent.gamma_multiply(0.6)),
                            egui::StrokeKind::Inside,
                        );
                        let dot_center = swatch_rect.center() + egui::vec2(5.0, 5.0);
                        ui.painter().circle_filled(dot_center, 4.0, swatch_accent);

                        let text_color = if selected {
                            theme::Colors::accent_light()
                        } else {
                            theme::Colors::text_primary()
                        };
                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.bottom() - 10.0),
                            egui::Align2::CENTER_CENTER,
                            &label,
                            egui::FontId::proportional(11.0),
                            text_color,
                        );

                        if response.clicked() && !selected {
                            if is_builtin {
                                self.theme_preset = theme::ThemePreset::ALL[idx];
                                self.active_custom_theme = None;
                                self.config.theme = theme::ThemePreset::ALL[idx];
                                self.config.custom_theme_name = None;
                            } else {
                                let ct = &custom_themes[idx - theme::ThemePreset::ALL.len()];
                                self.active_custom_theme = Some(ct.file_name.clone());
                                self.config.custom_theme_name = Some(ct.file_name.clone());
                            }
                            if let Err(e) = self.config.save() {
                                self.status = format!("✗ Save failed: {}", e);
                            }
                        }

                        if (idx + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(theme::muted(I18n::t(&lang, "custom_themes_hint")));

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(&lang, "language")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(I18n::t(&lang, "refresh")).clicked() {
                    I18n::load_external_locales(&self.config.locales_dir());
                }
            });
        });
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(&lang, "language_desc")));
            ui.add_space(12.0);

            let languages = I18n::available_languages();
            let cols = 3;
            let total_width = ui.available_width();
            let card_width = (total_width - 8.0 * (cols - 1) as f32) / cols as f32;

            egui::Grid::new("language_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 8.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    for (i, entry) in languages.iter().enumerate() {
                        let selected = self.language == entry.id;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 40.0),
                            egui::Sense::click(),
                        );

                        let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("lang_card").with(i).with("sel"),
                            selected,
                            0.25,
                            crate::animation::ease_linear,
                        );
                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("lang_card").with(i).with("hover"),
                            response.hovered(),
                            0.2,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = if sel_t > 0.0 {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::accent(),
                                0.15 * sel_t,
                            )
                        } else {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::bg_widget_hover(),
                                hover_t,
                            )
                        };

                        let stroke = if sel_t > 0.0 {
                            egui::Stroke::new(1.5, theme::Colors::accent().gamma_multiply(sel_t))
                        } else {
                            egui::Stroke::new(1.0, theme::Colors::subtle_border())
                        };

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let text_color = if selected {
                            theme::Colors::accent_light()
                        } else {
                            theme::Colors::text_primary()
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &entry.name,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        if response.clicked() && !selected {
                            self.language = entry.id.clone();
                            self.config.language = entry.id.clone();
                            if let Err(e) = self.config.save() {
                                self.status = format!("✗ Save failed: {}", e);
                            }
                        }

                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });
        });

        ui.add_space(8.0);
        ui.label(theme::muted(I18n::t(&lang, "language_hint")));
    }

    fn render_tab_data(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();

        // ── Data Directory ──────────────────────────────────────────────
        ui.label(theme::subheading(I18n::t(&lang, "data_dir")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(&lang, "data_dir_desc")));
            ui.add_space(8.0);

            let total_w = ui.available_width();
            let btn_width = 60.0;
            let input_width = total_w - btn_width - 8.0;

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.data_dir_input)
                        .vertical_align(egui::Align::Center)
                        .desired_width(input_width)
                        .min_size(ui.spacing().interact_size),
                );
                if ui.button(I18n::t(&lang, "browse")).clicked()
                    && let Some(folder) = rfd::FileDialog::new()
                        .set_title("Select data directory")
                        .pick_folder()
                {
                    self.data_dir_input = folder.display().to_string();
                }
            });
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(I18n::t(&lang, "apply_migrate")).clicked() {
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
                                    self.status = I18n::t(&lang, "data_dir_migrated").to_string();
                                } else {
                                    self.instances = miao_core::instance::list_instances(
                                        &self.config.instances_dir(),
                                    )
                                    .unwrap_or_default();
                                    self.selected_instance = None;
                                    self.status = I18n::t(&lang, "data_dir_partial").to_string();
                                }
                            }
                        }
                    }
                    if ui.button(I18n::t(&lang, "apply")).clicked() {
                        let new_path = std::path::PathBuf::from(&self.data_dir_input);
                        if let Err(e) = std::fs::create_dir_all(&new_path) {
                            self.status = format!("✗ Failed to create directory: {}", e);
                        } else {
                            self.config.data_dir = new_path;
                            if let Err(e) = self.config.save() {
                                self.status = format!("✗ Failed to save config: {}", e);
                            } else {
                                self.instances = miao_core::instance::list_instances(
                                    &self.config.instances_dir(),
                                )
                                .unwrap_or_default();
                                self.selected_instance = None;
                                self.status = I18n::t(&lang, "data_dir_updated").to_string();
                            }
                        }
                    }
                });
            });
        });

        // ── Game Folders ────────────────────────────────────────────────
        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(&lang, "game_folders")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(&lang, "game_folders_desc")));
            ui.add_space(10.0);

            let active = self.config.data_dir.display().to_string();
            let mut switch_to: Option<std::path::PathBuf> = None;
            let mut remove_idx: Option<usize> = None;

            for (i, folder) in self.config.game_folders.iter().enumerate() {
                let is_active = *folder == self.config.data_dir;
                ui.horizontal(|ui| {
                    if is_active {
                        ui.label(
                            egui::RichText::new(crate::icons::ICON_CIRCLE_FILL)
                                .size(8.0)
                                .color(theme::Colors::success()),
                        );
                        ui.label(theme::body(&folder.display().to_string()));
                    } else {
                        ui.label(theme::muted(&folder.display().to_string()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !is_active && ui.small_button("✕").clicked() {
                            remove_idx = Some(i);
                        }
                        if !is_active && ui.small_button(I18n::t(&lang, "switch")).clicked() {
                            switch_to = Some(folder.clone());
                        }
                    });
                });
                ui.add_space(4.0);
            }

            if self.config.game_folders.is_empty()
                || !self.config.game_folders.contains(&self.config.data_dir)
            {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(crate::icons::ICON_CIRCLE_FILL)
                            .size(8.0)
                            .color(theme::Colors::success()),
                    );
                    ui.label(theme::body(&active));
                });
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(I18n::t(&lang, "add_folder")).clicked()
                        && let Some(folder) = rfd::FileDialog::new()
                            .set_title("Select game folder")
                            .pick_folder()
                        && !self.config.game_folders.contains(&folder)
                    {
                        self.config.game_folders.push(folder);
                        let _ = self.config.save();
                    }
                });
            });

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
                self.status = I18n::t(&lang, "folder_switched").to_string();
            }
        });

        // ── Download Settings ───────────────────────────────────────────
        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(&lang, "mirror")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let current_mirror = match &self.config.download_mirror {
                DownloadMirror::Official => 0,
                DownloadMirror::Bmclapi => 1,
                DownloadMirror::Custom(_) => 2,
            };
            let mut selected = current_mirror;

            let mirror_labels = [
                I18n::t(&lang, "mirror_official").to_string(),
                I18n::t(&lang, "mirror_bmclapi").to_string(),
                I18n::t(&lang, "mirror_custom").to_string(),
            ];

            let cols = 3;
            let total_width = ui.available_width();
            let card_width = (total_width - 8.0 * (cols - 1) as f32) / cols as f32;

            egui::Grid::new("mirror_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 8.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    for (i, label) in mirror_labels.iter().enumerate() {
                        let is_selected = i == current_mirror;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 36.0),
                            egui::Sense::click(),
                        );

                        let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("mirror_card").with(i).with("sel"),
                            is_selected,
                            0.25,
                            crate::animation::ease_linear,
                        );
                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("mirror_card").with(i).with("hover"),
                            response.hovered(),
                            0.2,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = if sel_t > 0.0 {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::accent(),
                                0.15 * sel_t,
                            )
                        } else {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::bg_widget_hover(),
                                hover_t,
                            )
                        };

                        let stroke = if sel_t > 0.0 {
                            egui::Stroke::new(1.5, theme::Colors::accent().gamma_multiply(sel_t))
                        } else {
                            egui::Stroke::new(1.0, theme::Colors::subtle_border())
                        };

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let text_color = if is_selected {
                            theme::Colors::accent_light()
                        } else {
                            theme::Colors::text_primary()
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        if response.clicked() && !is_selected {
                            selected = i;
                        }

                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });

            if selected == 2 {
                ui.add_space(10.0);
                ui.label(theme::small("URL"));
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.mirror_custom_url)
                        .vertical_align(egui::Align::Center)
                        .desired_width(ui.available_width())
                        .hint_text("https://my-mirror.example.com"),
                );
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
                    self.status = I18n::t(&lang, "mirror_updated").to_string();
                }
            }

            // ── Max concurrent downloads ────────────────────────────────
            ui.add_space(16.0);
            ui.label(theme::small(I18n::t(&lang, "max_concurrent")));
            ui.add_space(4.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.max_downloads_input)
                    .desired_width(80.0)
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

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(&lang, "cf_api_key")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(theme::muted(I18n::t(&lang, "cf_api_key_desc")));
            ui.add_space(8.0);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut self.cf_api_key_input)
                    .password(true)
                    .vertical_align(egui::Align::Center)
                    .desired_width(ui.available_width())
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
    }

    fn render_tab_java(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();

        // ── Java source ────────────────────────────────────────────────────
        ui.label(theme::subheading(I18n::t(&lang, "java_source")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            let sources = miao_core::config::JavaSource::all();
            let current_idx = sources
                .iter()
                .position(|s| *s == self.config.java_source)
                .unwrap_or(0);
            let mut selected_idx = current_idx;

            let cols = sources.len().clamp(1, 3);
            let total_width = ui.available_width();
            let card_width = if cols > 1 {
                (total_width - 8.0 * (cols - 1) as f32) / cols as f32
            } else {
                total_width
            };

            egui::Grid::new("java_source_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 8.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    for (i, source) in sources.iter().enumerate() {
                        let is_selected = i == current_idx;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 36.0),
                            egui::Sense::click(),
                        );

                        let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("java_source_card").with(i).with("sel"),
                            is_selected,
                            0.25,
                            crate::animation::ease_linear,
                        );
                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("java_source_card").with(i).with("hover"),
                            response.hovered(),
                            0.2,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = if sel_t > 0.0 {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::accent(),
                                0.15 * sel_t,
                            )
                        } else {
                            crate::animation::lerp_color(
                                base,
                                theme::Colors::bg_widget_hover(),
                                hover_t,
                            )
                        };

                        let stroke = if sel_t > 0.0 {
                            egui::Stroke::new(1.5, theme::Colors::accent().gamma_multiply(sel_t))
                        } else {
                            egui::Stroke::new(1.0, theme::Colors::subtle_border())
                        };

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            stroke,
                            egui::StrokeKind::Inside,
                        );

                        let text_color = if is_selected {
                            theme::Colors::accent_light()
                        } else {
                            theme::Colors::text_primary()
                        };
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            source_display_label(&lang, *source),
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        let desc = source_description(&lang, *source);
                        let response = response.on_hover_text(desc);

                        if response.clicked() && !is_selected {
                            selected_idx = i;
                        }

                        if (i + 1) % cols == 0 {
                            ui.end_row();
                        }
                    }
                });

            if selected_idx != current_idx
                && let Some(new_source) = sources.get(selected_idx).copied()
            {
                self.config.java_source = new_source;
                if let Err(e) = self.config.save() {
                    self.status = format!("✗ Save failed: {}", e);
                } else {
                    self.status = I18n::t(&lang, "java_source_updated").to_string();
                }
            }
        });

        ui.add_space(16.0);

        // ── Detected installations ────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(&lang, "java_installs")));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh_btn = ui.add_enabled(
                    !self.java_detecting,
                    egui::Button::new(I18n::t(&lang, "refresh")),
                );
                if refresh_btn.clicked() {
                    self.cached_javas = None;
                    self.request_java_detection(true);
                }
            });
        });
        ui.add_space(8.0);

        // Kick off a background scan if we don't yet have any data and one isn't
        // already in flight. UI keeps rendering with a spinner placeholder.
        if self.cached_javas.is_none() && !self.java_detecting {
            self.request_java_detection(false);
        }

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            if let Some(ref javas) = self.cached_javas {
                if javas.is_empty() {
                    ui.label(theme::muted("No Java installations detected."));
                    ui.add_space(4.0);
                    ui.label(theme::small(
                        "Install Java or use 'Download Java' from an instance.",
                    ));
                } else {
                    for (i, j) in javas.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("Java {}", j.major_version))
                                    .strong()
                                    .color(theme::Colors::text_primary()),
                            );
                            ui.label(theme::muted(&format!("({})", j.version)));
                        });
                        ui.label(theme::small(&j.path.display().to_string()));
                        if i < javas.len() - 1 {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.add_space(6.0);
                        }
                    }
                }
            } else {
                // First-time detection in progress.
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(theme::muted(I18n::t(&lang, "detecting_java")));
                });
            }
        });
    }

    fn render_tab_help(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();

        ui.label(theme::subheading(I18n::t(&lang, "faq")));
        ui.add_space(8.0);

        let faqs: &[(&str, &str)] = if lang == "zh" {
            &[
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
            ]
        } else {
            &[
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
            ]
        };

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            for (i, (question, answer)) in faqs.iter().enumerate() {
                crate::widgets::CollapsibleCard::new(question, question)
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label(theme::body(answer));
                    });
                if i < faqs.len() - 1 {
                    ui.add_space(6.0);
                }
            }
        });
    }

    fn render_tab_about(&mut self, ui: &mut egui::Ui) {
        let lang = self.language.clone();
        ui.label(theme::subheading(I18n::t(&lang, "about")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("MMCL")
                    .size(24.0)
                    .strong()
                    .color(theme::Colors::accent_light()),
            );
            ui.add_space(2.0);
            ui.label(theme::muted(&format!(
                "MiaoMinecraftLauncher v{}",
                env!("CARGO_PKG_VERSION")
            )));
            ui.add_space(6.0);
            ui.label(theme::body(
                "A feature-rich cross-platform Minecraft launcher.",
            ));
            ui.add_space(4.0);
            ui.label(theme::muted("License: GPL-3.0-or-later"));
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
                            I18n::t(&self.language, "update_available"),
                            version
                        )));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if ui.button(I18n::t(&lang, "download")).clicked() {
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
        ui.label(theme::subheading(I18n::t(&lang, "author")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                egui::RichText::new("MickeyMiao")
                    .strong()
                    .color(theme::Colors::text_primary()),
            );
            ui.add_space(10.0);

            let cols = 2;
            let total_width = ui.available_width();
            let card_width = (total_width - 8.0) / cols as f32;

            egui::Grid::new("author_links_grid")
                .num_columns(cols)
                .spacing(egui::vec2(8.0, 8.0))
                .min_col_width(card_width)
                .max_col_width(card_width)
                .show(ui, |ui| {
                    let links: [(&str, &str, &str); 2] = [
                        (
                            "GitHub",
                            "WangSimiao2000",
                            "https://github.com/WangSimiao2000/MiaoMinecraftLauncher",
                        ),
                        (
                            "Bilibili",
                            "鄙人米奇喵",
                            "https://space.bilibili.com/36913332",
                        ),
                    ];

                    for (platform, name, url) in links {
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(card_width, 40.0),
                            egui::Sense::click(),
                        );

                        let hover_t = ui.ctx().animate_bool_with_time_and_easing(
                            egui::Id::new("author_link").with(platform).with("hover"),
                            response.hovered(),
                            0.2,
                            crate::animation::ease_linear,
                        );

                        let base = theme::Colors::bg_widget();
                        let bg = crate::animation::lerp_color(
                            base,
                            theme::Colors::bg_widget_hover(),
                            hover_t,
                        );

                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(6), bg);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(6),
                            egui::Stroke::new(1.0, theme::Colors::subtle_border()),
                            egui::StrokeKind::Inside,
                        );

                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.center().y - 7.0),
                            egui::Align2::CENTER_CENTER,
                            platform,
                            egui::FontId::proportional(11.0),
                            theme::Colors::text_secondary(),
                        );
                        ui.painter().text(
                            egui::pos2(rect.center().x, rect.center().y + 7.0),
                            egui::Align2::CENTER_CENTER,
                            name,
                            egui::FontId::proportional(12.0),
                            theme::Colors::accent_light(),
                        );

                        if response.clicked() {
                            let _ = open::that(url);
                        }
                    }
                });
        });

        ui.add_space(16.0);
        ui.label(theme::subheading(I18n::t(&lang, "open_source_credits")));
        ui.add_space(8.0);

        theme::section_frame().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.label(theme::small("Fonts"));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(theme::muted("·"));
                ui.hyperlink_to(
                    "MiSans — Xiaomi (SIL OFL 1.1)",
                    "https://hyperos.mi.com/font/en",
                );
            });
            ui.horizontal(|ui| {
                ui.label(theme::muted("·"));
                ui.hyperlink_to(
                    "Bootstrap Icons — The Bootstrap Authors (MIT)",
                    "https://icons.getbootstrap.com/",
                );
            });
            ui.add_space(12.0);
            ui.label(theme::small("Core Libraries"));
            ui.add_space(4.0);
            for (name, url) in [
                (
                    "egui / eframe — emilk (MIT OR Apache-2.0)",
                    "https://github.com/emilk/egui",
                ),
                (
                    "tokio — Tokio Contributors (MIT)",
                    "https://github.com/tokio-rs/tokio",
                ),
                (
                    "reqwest — seanmonstar (MIT OR Apache-2.0)",
                    "https://github.com/seanmonstar/reqwest",
                ),
                (
                    "serde — David Tolnay (MIT OR Apache-2.0)",
                    "https://github.com/serde-rs/serde",
                ),
                (
                    "clap — Kevin K. (MIT OR Apache-2.0)",
                    "https://github.com/clap-rs/clap",
                ),
                (
                    "egui_animation — lucasmerlin (MIT)",
                    "https://github.com/lucasmerlin/hello_egui",
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(theme::muted("·"));
                    ui.hyperlink_to(name, url);
                });
            }
        });
    }
}

fn source_display_label(_lang: &str, source: JavaSource) -> &'static str {
    // Display name comes from the source itself for now. If we ever want localised
    // aliases, swap this for an i18n key like
    // `format!("java_source_{}_label", source.id())`.
    source.display_name()
}

fn source_description(lang: &str, source: JavaSource) -> &'static str {
    let key: &'static str = match source {
        JavaSource::Mojang => "java_source_mojang_desc",
        JavaSource::Bmclapi => "java_source_bmclapi_desc",
        JavaSource::Adoptium => "java_source_adoptium_desc",
        JavaSource::Microsoft => "java_source_microsoft_desc",
    };
    I18n::t(lang, key)
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
