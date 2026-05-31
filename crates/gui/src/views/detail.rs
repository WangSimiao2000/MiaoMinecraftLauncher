use eframe::egui;

use crate::animation::lerp_color;
use crate::app::{DetailTab, I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_detail(&mut self, ui: &mut egui::Ui) {
        let has_instance = self.selected_instance.is_some();
        let detail_opacity = ui.ctx().animate_bool_with_time_and_easing(
            egui::Id::new("detail_content_opacity"),
            has_instance,
            0.3,
            crate::animation::ease_linear,
        );
        let welcome_opacity = 1.0 - detail_opacity;

        if welcome_opacity > 0.0 && !has_instance {
            ui.set_opacity(welcome_opacity);
            self.render_welcome(ui);
            if detail_opacity == 0.0 {
                return;
            }
        }

        let Some(idx) = self.selected_instance else {
            return;
        };

        let inst = self.instances[idx].clone();
        let instance_dir =
            miao_core::instance::Instance::instance_dir(&self.config.instances_dir(), &inst.name);

        ui.set_opacity(detail_opacity);
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
            DetailTab::ModpackSync => self.render_modpack_sync_tab(ui, &instance_dir, &inst),
            DetailTab::Settings => self.render_instance_settings_tab(ui),
        });
    }

    fn render_welcome(&self, ui: &mut egui::Ui) {
        ui.add_space(100.0);
        ui.vertical_centered(|ui| {
            ui.label(theme::heading(I18n::t(&self.language, "welcome")));
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.label(theme::muted(I18n::t(&self.language, "welcome_hint")));
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
                                .unwrap_or_else(|| I18n::t(&self.language, "vanilla").to_string());
                            ui.label(theme::badge_loader(&loader));
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let launch_task_id = format!("launch:{}", inst.name);
                        let is_launching = self.active_installs.contains(&launch_task_id);
                        let btn_label = if is_launching {
                            I18n::t(&self.language, "launching_short")
                        } else {
                            I18n::t(&self.language, "launch")
                        };
                        if ui
                            .add_enabled(!is_launching, theme::launch_button(btn_label))
                            .clicked()
                        {
                            self.launch_instance(idx);
                        }
                        ui.add_space(8.0);
                        if ui.button(I18n::t(&self.language, "export")).clicked() {
                            self.export_instance_with_dialog(idx);
                        }
                        if ui.button(I18n::t(&self.language, "open")).clicked() {
                            let dir = miao_core::instance::Instance::instance_dir(
                                &self.config.instances_dir(),
                                &inst.name,
                            );
                            let _ = miao_core::instance::open_folder(&dir);
                        }
                        ui.add_space(8.0);
                        if self.confirm_delete == Some(idx) {
                            ui.label(
                                egui::RichText::new(I18n::t(&self.language, "confirm"))
                                    .color(theme::Colors::danger())
                                    .size(12.0),
                            );
                            if ui
                                .add(theme::danger_button(I18n::t(&self.language, "yes")))
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
                            if ui.button(I18n::t(&self.language, "no")).clicked() {
                                self.confirm_delete = None;
                            }
                        } else if ui
                            .add(theme::danger_button(I18n::t(&self.language, "delete")))
                            .clicked()
                        {
                            self.confirm_delete = Some(idx);
                        }
                    });
                });
            });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        let show_modpack_sync = self
            .selected_instance
            .and_then(|i| self.instances.get(i))
            .map(|inst| inst.modpack_subscription.is_some())
            .unwrap_or(false);

        let mut tab_list: Vec<(DetailTab, &str)> = vec![
            (DetailTab::Mods, "tab_mods"),
            (DetailTab::Resources, "tab_resources"),
            (DetailTab::Worlds, "tab_worlds"),
            (DetailTab::Log, "tab_log"),
        ];
        if show_modpack_sync {
            tab_list.push((DetailTab::ModpackSync, "tab_modpack_sync"));
        }
        tab_list.push((DetailTab::Settings, "tab_settings"));

        ui.horizontal(|ui| {
            let lang = self.language.clone();
            for (tab, key) in tab_list {
                let label = I18n::t(&lang, key);
                let selected = self.active_tab == tab;

                let sel_t = ui.ctx().animate_bool_with_time_and_easing(
                    egui::Id::new(key).with("tab_sel"),
                    selected,
                    0.25,
                    crate::animation::ease_linear,
                );

                let active_color = theme::Colors::accent_light();
                let idle_color = theme::Colors::text_secondary();
                let text_color = lerp_color(idle_color, active_color, sel_t);

                let text = egui::RichText::new(label).color(text_color);
                let response = ui.selectable_label(false, text);

                if selected {
                    let rect = response.rect;
                    let target_x = rect.left() + 2.0;
                    let target_w = rect.width() - 4.0;
                    if self.tab_indicator_x.position() == 0.0
                        && self.tab_indicator_x.velocity() == 0.0
                    {
                        self.tab_indicator_x.snap(target_x);
                        self.tab_indicator_width.snap(target_w);
                    } else {
                        self.tab_indicator_x.set_target(target_x);
                        self.tab_indicator_width.set_target(target_w);
                    }
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

            let indicator_x = self.tab_indicator_x.position();
            let indicator_w = self.tab_indicator_width.position();
            let row_rect = ui.min_rect();
            let bottom = row_rect.bottom();
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(indicator_x, bottom - theme::TAB_UNDERLINE_HEIGHT),
                    egui::vec2(indicator_w, theme::TAB_UNDERLINE_HEIGHT),
                ),
                egui::CornerRadius::same(2),
                theme::Colors::accent(),
            );
        });
    }
}
