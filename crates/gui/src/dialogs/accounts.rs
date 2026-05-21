use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;

use crate::app::{AsyncState, Dialog, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_accounts_dialog(&mut self, ctx: &egui::Context, state: &AsyncState) {
        let mut open = true;
        egui::Window::new("Accounts")
            .open(&mut open)
            .resizable(false)
            .default_width(400.0)
            .show(ctx, |ui| {
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
                            AuthMethod::Microsoft(a) => {
                                format!("{}{} (Microsoft)", prefix, a.username)
                            }
                        };
                        if ui.selectable_label(active, &name).clicked() {
                            self.config.active_account_index = Some(i);
                            let _ = self.config.save();
                        }
                    }
                }

                ui.add_space(theme::Spacing::SECTION_GAP);
                ui.separator();
                ui.add_space(theme::Spacing::SMALL_GAP);

                ui.label(theme::body("Add account:"));
                ui.add_space(theme::Spacing::SMALL_GAP);

                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.offline_username_input);
                    if ui.button("Add Offline").clicked() && !self.offline_username_input.is_empty()
                    {
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

                if state.ms_logging_in {
                    if let Some(ref dc) = state.ms_device_code {
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
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }
}
