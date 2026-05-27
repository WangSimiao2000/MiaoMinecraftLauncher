use eframe::egui;

use crate::app::{DetailTab, I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_detail(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected_instance else {
            self.render_welcome(ui);
            return;
        };

        let inst = self.instances[idx].clone();
        let instance_dir =
            miao_core::instance::Instance::instance_dir(&self.config.instances_dir(), &inst.name);

        self.render_instance_header(ui, &inst, idx);
        ui.add_space(theme::Spacing::SECTION_GAP);
        self.render_tabs(ui);
        ui.add_space(theme::Spacing::SMALL_GAP);
        ui.separator();
        ui.add_space(theme::Spacing::SMALL_GAP);

        egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
            DetailTab::Mods => self.render_mods_tab(ui, &instance_dir),
            DetailTab::Resources => self.render_resources_tab(ui, &instance_dir),
            DetailTab::Worlds => self.render_worlds_tab(ui, &instance_dir),
            DetailTab::Log => self.render_log_tab(ui, &instance_dir),
            DetailTab::Settings => self.render_instance_settings_tab(ui),
        });
    }

    fn render_welcome(&self, ui: &mut egui::Ui) {
        ui.add_space(100.0);
        ui.vertical_centered(|ui| {
            ui.label(theme::heading(I18n::t(self.language, "welcome")));
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.label(theme::muted(I18n::t(self.language, "welcome_hint")));
        });
    }

    fn render_instance_header(
        &mut self,
        ui: &mut egui::Ui,
        inst: &miao_core::instance::Instance,
        idx: usize,
    ) {
        egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(theme::Radii::WIDGET)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(theme::title(&inst.name));
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(theme::badge_mc(&inst.minecraft_version));
                            ui.add_space(8.0);
                            let loader = inst
                                .mod_loader
                                .as_ref()
                                .map(|l| format!("{} {}", l.loader_type, l.version))
                                .unwrap_or_else(|| I18n::t(self.language, "vanilla").to_string());
                            ui.label(theme::badge_loader(&loader));
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(theme::launch_button()).clicked() {
                            self.launch_instance(idx);
                        }
                        ui.add_space(8.0);
                        if ui.button(I18n::t(self.language, "export")).clicked() {
                            self.export_instance_with_dialog(idx);
                        }
                        if ui.button(I18n::t(self.language, "open")).clicked() {
                            let dir = miao_core::instance::Instance::instance_dir(
                                &self.config.instances_dir(),
                                &inst.name,
                            );
                            let _ = miao_core::instance::open_folder(&dir);
                        }
                        ui.add_space(8.0);
                        if self.confirm_delete == Some(idx) {
                            ui.label(
                                egui::RichText::new(I18n::t(self.language, "confirm"))
                                    .color(theme::Colors::danger())
                                    .size(12.0),
                            );
                            if ui
                                .add(theme::danger_button(I18n::t(self.language, "yes")))
                                .clicked()
                            {
                                let name = inst.name.clone();
                                if let Err(e) = miao_core::instance::delete_instance(
                                    &self.config.instances_dir(),
                                    &name,
                                ) {
                                    self.status = format!("Delete failed: {}", e);
                                } else {
                                    self.status = format!("Deleted '{}'", name);
                                    self.instances = miao_core::instance::list_instances(
                                        &self.config.instances_dir(),
                                    )
                                    .unwrap_or_default();
                                    self.selected_instance = None;
                                }
                                self.confirm_delete = None;
                            }
                            if ui.button(I18n::t(self.language, "no")).clicked() {
                                self.confirm_delete = None;
                            }
                        } else if ui
                            .add(theme::danger_button(I18n::t(self.language, "delete")))
                            .clicked()
                        {
                            self.confirm_delete = Some(idx);
                        }
                    });
                });
            });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let lang = self.language;
            for (tab, key) in [
                (DetailTab::Mods, "tab_mods"),
                (DetailTab::Resources, "tab_resources"),
                (DetailTab::Worlds, "tab_worlds"),
                (DetailTab::Log, "tab_log"),
                (DetailTab::Settings, "tab_settings"),
            ] {
                let label = I18n::t(lang, key);
                let selected = self.active_tab == tab;
                let text = if selected {
                    egui::RichText::new(label)
                        .strong()
                        .color(theme::Colors::accent_light())
                } else {
                    egui::RichText::new(label).color(theme::Colors::text_secondary())
                };

                let response = ui.selectable_label(false, text);

                if selected {
                    let rect = response.rect;
                    let bottom = rect.bottom();
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left() + 2.0, bottom - theme::TAB_UNDERLINE_HEIGHT),
                            egui::pos2(rect.right() - 2.0, bottom),
                        ),
                        egui::CornerRadius::same(2),
                        theme::Colors::accent(),
                    );
                }

                if response.clicked() {
                    if tab == DetailTab::Settings
                        && self.active_tab != DetailTab::Settings
                        && let Some(idx) = self.selected_instance
                    {
                        self.load_instance_settings_edit(idx);
                    }
                    self.active_tab = tab;
                }
            }
        });
    }
}
