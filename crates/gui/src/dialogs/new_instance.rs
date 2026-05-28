use eframe::egui;

use crate::app::{Dialog, I18n, MiaoApp};
use crate::theme;

impl MiaoApp {
    pub fn render_new_instance_dialog(&mut self, ctx: &egui::Context) {
        if !self.versions.versions.is_empty()
            && self.loader.versions.is_empty()
            && !self.loader.loading
        {
            let mc_ver = self.versions.versions[self.new_instance.version_idx]
                .id
                .clone();
            self.fetch_loader_versions(&mc_ver);
        }

        let lang = self.language.clone();
        let mut open = true;

        let anim_id = egui::Id::new("new_instance_dialog_anim");
        let t =
            ctx.animate_bool_with_time_and_easing(anim_id, true, 0.3, crate::animation::ease_out);
        let opacity = t;
        let scale_offset = (1.0 - t) * 8.0;

        let frame = egui::Frame::window(&ctx.global_style())
            .fill(theme::Colors::bg_elevated())
            .stroke(egui::Stroke::new(
                1.0,
                theme::Colors::accent().gamma_multiply(0.5),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(16));

        egui::Window::new(I18n::t(&lang, "create_instance"))
            .title_bar(false)
            .resizable(false)
            .default_width(420.0)
            .max_width(420.0)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, scale_offset])
            .order(egui::Order::Foreground)
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_opacity(opacity);
                ui.horizontal(|ui| {
                    ui.label(theme::subheading(I18n::t(&lang, "create_instance")));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_size = egui::vec2(28.0, 28.0);
                        let (rect, response) =
                            ui.allocate_exact_size(btn_size, egui::Sense::click());
                        let color = if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                egui::Color32::from_rgb(196, 43, 28),
                            );
                            egui::Color32::WHITE
                        } else {
                            theme::Colors::text_secondary()
                        };
                        let center = rect.center();
                        let d = 5.0;
                        let stroke = egui::Stroke::new(1.5, color);
                        ui.painter().line_segment(
                            [center - egui::vec2(d, d), center + egui::vec2(d, d)],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [center + egui::vec2(-d, d), center + egui::vec2(d, -d)],
                            stroke,
                        );
                        if response.clicked() {
                            open = false;
                        }
                    });
                });
                ui.add_space(theme::Spacing::SMALL_GAP);

                ui.horizontal(|ui| {
                    ui.label(I18n::t(&lang, "instance_name"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_instance.name)
                            .vertical_align(egui::Align::Center)
                            .min_size(ui.spacing().interact_size),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(I18n::t(&lang, "mc_version"));
                    if !self.versions.versions.is_empty() {
                        let current = self
                            .versions
                            .versions
                            .get(self.new_instance.version_idx)
                            .map(|v| v.id.as_str())
                            .unwrap_or("?");
                        let prev_idx = self.new_instance.version_idx;
                        egui::ComboBox::from_id_salt("mc_ver")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (i, ver) in self.versions.versions.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.new_instance.version_idx,
                                        i,
                                        &ver.id,
                                    );
                                }
                            });
                        if self.new_instance.version_idx != prev_idx {
                            self.new_instance.loader = 0;
                            self.new_instance.loader_version_idx = 0;
                            let mc_ver = self.versions.versions[self.new_instance.version_idx]
                                .id
                                .clone();
                            self.fetch_loader_versions(&mc_ver);
                        }
                    } else {
                        ui.spinner();
                        ui.label(I18n::t(&lang, "loading"));
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(theme::small(I18n::t(&lang, "show")));
                    ui.add_space(4.0);
                    let mut show_snap = self.versions.show_snapshots;
                    let mut show_beta = self.versions.show_old_beta;
                    let mut show_alpha = self.versions.show_old_alpha;

                    if ui
                        .checkbox(&mut show_snap, I18n::t(&lang, "show_snapshots"))
                        .changed()
                    {
                        self.update_version_filter(show_snap, show_beta, show_alpha);
                    }
                    if ui
                        .checkbox(&mut show_beta, I18n::t(&lang, "show_old_beta"))
                        .changed()
                    {
                        self.update_version_filter(show_snap, show_beta, show_alpha);
                    }
                    if ui
                        .checkbox(&mut show_alpha, I18n::t(&lang, "show_old_alpha"))
                        .changed()
                    {
                        self.update_version_filter(show_snap, show_beta, show_alpha);
                    }
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(I18n::t(&lang, "mod_loader"));
                    let loaders = self.get_available_loaders();
                    let current_name = loaders
                        .iter()
                        .find(|(idx, _, _)| *idx == self.new_instance.loader)
                        .map(|(_, name, _)| *name)
                        .unwrap_or("None (Vanilla)");

                    egui::ComboBox::from_id_salt("loader")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for (idx, name, available) in &loaders {
                                ui.add_enabled_ui(*available, |ui| {
                                    let label = if *available {
                                        name.to_string()
                                    } else {
                                        format!("{} (N/A)", name)
                                    };
                                    ui.selectable_value(&mut self.new_instance.loader, *idx, label);
                                });
                            }
                        });
                });

                if self.new_instance.loader > 0 {
                    let loader_versions: Vec<_> =
                        self.get_loader_versions().into_iter().cloned().collect();
                    if !loader_versions.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label(I18n::t(&lang, "loader_version"));
                            let current = loader_versions
                                .get(self.new_instance.loader_version_idx)
                                .map(|v| v.version.as_str())
                                .unwrap_or("?");
                            egui::ComboBox::from_id_salt("loader_ver")
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    for (i, v) in loader_versions.iter().enumerate() {
                                        let label = if v.stable {
                                            format!("{} (stable)", v.version)
                                        } else {
                                            v.version.clone()
                                        };
                                        ui.selectable_value(
                                            &mut self.new_instance.loader_version_idx,
                                            i,
                                            label,
                                        );
                                    }
                                });
                        });
                    } else if self.loader.loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(I18n::t(&lang, "loading"));
                        });
                    }
                }

                ui.add_space(theme::Spacing::SECTION_GAP);

                let can_create = !self.versions.versions.is_empty();
                ui.add_enabled_ui(can_create, |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(I18n::t(&self.language, "create")).clicked() {
                            let ver = self.versions.versions[self.new_instance.version_idx].clone();
                            let name = if self.new_instance.name.is_empty() {
                                ver.id.clone()
                            } else {
                                self.new_instance.name.clone()
                            };

                            let loader = if self.new_instance.loader > 0 {
                                let lt = miao_core::modloader::ModLoaderType::from_index(
                                    self.new_instance.loader - 1,
                                );
                                let lv = self
                                    .get_loader_versions()
                                    .get(self.new_instance.loader_version_idx)
                                    .map(|v| v.version.clone());
                                match (lt, lv) {
                                    (Some(lt), Some(lv)) => Some((lt.as_str().to_string(), lv)),
                                    _ => None,
                                }
                            } else {
                                None
                            };

                            self.create_instance(ver, name, loader);
                            self.active_dialog = Dialog::None;
                        }
                    });
                });
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }
}
