use eframe::egui;

use crate::app::{DetailTab, MiaoApp};
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
        });
    }

    fn render_welcome(&self, ui: &mut egui::Ui) {
        ui.add_space(100.0);
        ui.vertical_centered(|ui| {
            ui.label(theme::heading("Welcome to MMCL"));
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.label(theme::muted(
                "Select an instance from the left panel,\nor click '+ New' to create one.",
            ));
        });
    }

    fn render_instance_header(
        &mut self,
        ui: &mut egui::Ui,
        inst: &miao_core::instance::Instance,
        idx: usize,
    ) {
        egui::Frame::none()
            .fill(theme::Colors::BG_ELEVATED)
            .rounding(theme::Radii::WIDGET)
            .inner_margin(egui::Margin::same(12.0))
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
                                .unwrap_or_else(|| "Vanilla".to_string());
                            ui.label(theme::badge_loader(&loader));
                        });
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(theme::launch_button()).clicked() {
                            self.launch_instance(idx);
                        }
                        ui.add_space(8.0);
                        if ui.button("Export").clicked() {
                            self.export_instance_with_dialog(idx);
                        }
                        if ui.button("Open").clicked() {
                            let dir = miao_core::instance::Instance::instance_dir(
                                &self.config.instances_dir(),
                                &inst.name,
                            );
                            let _ = miao_core::instance::open_folder(&dir);
                        }
                        ui.add_space(8.0);
                        if ui.add(theme::danger_button("Delete")).clicked() {
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
                        }
                    });
                });
            });
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for (tab, label) in [
                (DetailTab::Mods, "Mods"),
                (DetailTab::Resources, "Resources"),
                (DetailTab::Worlds, "Worlds"),
                (DetailTab::Log, "Log"),
            ] {
                let selected = self.active_tab == tab;
                let text = if selected {
                    egui::RichText::new(label)
                        .strong()
                        .color(theme::Colors::ACCENT_LIGHT)
                } else {
                    egui::RichText::new(label).color(theme::Colors::TEXT_SECONDARY)
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
                        egui::Rounding::same(1.5),
                        theme::Colors::ACCENT,
                    );
                }

                if response.clicked() {
                    self.active_tab = tab;
                }
            }
        });
    }
}
