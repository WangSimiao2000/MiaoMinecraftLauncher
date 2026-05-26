use eframe::egui;
use std::path::Path;

use crate::app::{I18n, MiaoApp};
use crate::messages::AppCommand;
use crate::state::ModSource;
use crate::theme;
use crate::widgets::{ModCard, mod_card::ModCardAction};

impl MiaoApp {
    pub fn render_mods_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        self.file_scan_cache.get_or_scan(instance_dir);
        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        let mods = self.file_scan_cache.mods.clone();

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!("Mods ({})", mods.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(I18n::t(lang, "open_folder")).clicked() {
                    let _ = miao_core::instance::open_folder(&mods_dir);
                }

                let search_label = if self.mod_search_active {
                    egui::RichText::new(format!(
                        "{} {}",
                        crate::icons::ICON_X_CIRCLE,
                        I18n::t(lang, "close_search")
                    ))
                    .color(theme::Colors::text_secondary())
                } else {
                    egui::RichText::new(format!(
                        "{} {}",
                        crate::icons::ICON_SEARCH,
                        I18n::t(lang, "search_mods")
                    ))
                    .color(theme::Colors::text_primary())
                };
                if ui.button(search_label).clicked() {
                    self.mod_search_active = !self.mod_search_active;
                    if self.mod_search_active && !self.mod_search_query.is_empty() {
                        self.do_unified_search(instance_dir);
                    }
                }
            });
        });

        ui.add_space(theme::Spacing::SMALL_GAP);

        if self.mod_search_active {
            self.render_unified_search(ui, instance_dir);
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.separator();
            ui.add_space(theme::Spacing::SMALL_GAP);
        }

        if let Some(ref pending) = self.pending_mod_install.clone() {
            self.render_install_confirmation(ui, pending);
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.separator();
            ui.add_space(theme::Spacing::SMALL_GAP);
        }

        if mods.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(lang, "no_mods")));
                ui.add_space(8.0);
                ui.label(theme::small(I18n::t(lang, "no_mods_hint")));
            });
        } else {
            for mut m in mods {
                match ModCard::new(&m.name, m.enabled).show(ui) {
                    ModCardAction::Toggle => {
                        let _ = m.toggle();
                    }
                    ModCardAction::Delete => {
                        let _ = m.delete();
                    }
                    ModCardAction::None => {}
                }
                ui.add_space(2.0);
            }
        }
    }

    fn render_unified_search(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        ui.horizontal(|ui| {
            let mr_selected = self.mod_source == ModSource::Modrinth;
            let cf_selected = self.mod_source == ModSource::CurseForge;

            let mr_btn = if mr_selected {
                egui::Button::new(egui::RichText::new("Modrinth").color(egui::Color32::WHITE))
                    .fill(theme::Colors::accent())
                    .rounding(egui::Rounding {
                        nw: 6.0,
                        sw: 6.0,
                        ne: 0.0,
                        se: 0.0,
                    })
            } else {
                egui::Button::new("Modrinth").rounding(egui::Rounding {
                    nw: 6.0,
                    sw: 6.0,
                    ne: 0.0,
                    se: 0.0,
                })
            };
            if ui.add(mr_btn).clicked() && !mr_selected {
                self.mod_source = ModSource::Modrinth;
                self.on_source_switched(instance_dir);
            }

            let cf_btn = if cf_selected {
                egui::Button::new(egui::RichText::new("CurseForge").color(egui::Color32::WHITE))
                    .fill(egui::Color32::from_rgb(240, 100, 30))
                    .rounding(egui::Rounding {
                        nw: 0.0,
                        sw: 0.0,
                        ne: 6.0,
                        se: 6.0,
                    })
            } else {
                egui::Button::new("CurseForge").rounding(egui::Rounding {
                    nw: 0.0,
                    sw: 0.0,
                    ne: 6.0,
                    se: 6.0,
                })
            };
            if ui.add(cf_btn).clicked() && !cf_selected {
                self.mod_source = ModSource::CurseForge;
                self.on_source_switched(instance_dir);
            }
        });

        ui.add_space(6.0);

        if self.mod_source == ModSource::CurseForge && self.effective_cf_api_key().is_empty() {
            ui.label(theme::muted(I18n::t(lang, "cf_no_key")));
            return;
        }

        ui.horizontal(|ui| {
            ui.label(I18n::t(lang, "search"));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.mod_search_query)
                    .desired_width(ui.available_width() - 60.0)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.do_unified_search(instance_dir);
            }
            if ui.button(I18n::t(lang, "go")).clicked() {
                self.do_unified_search(instance_dir);
            }
        });

        match self.mod_source {
            ModSource::Modrinth => self.render_modrinth_results(ui, instance_dir),
            ModSource::CurseForge => self.render_cf_results(ui, instance_dir),
        }
    }

    fn on_source_switched(&mut self, instance_dir: &Path) {
        if !self.mod_search_query.is_empty() {
            self.do_unified_search(instance_dir);
        }
    }

    fn do_unified_search(&mut self, instance_dir: &Path) {
        if self.mod_search_query.is_empty() {
            return;
        }
        match self.mod_source {
            ModSource::Modrinth => {
                self.mod_search.query = self.mod_search_query.clone();
                self.mod_search.active = true;
                self.mod_search.results.clear();
                self.mod_search.versions.clear();
                self.mod_search.selected = None;
                self.do_mod_search_from_tab();
            }
            ModSource::CurseForge => {
                self.cf_search.query = self.mod_search_query.clone();
                self.cf_search.active = true;
                self.cf_search.results.clear();
                self.do_cf_search(instance_dir);
            }
        }
    }

    fn render_modrinth_results(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        if self.mod_search.searching {
            ui.spinner();
        }

        if !self.mod_search.results.is_empty() && self.mod_search.versions.is_empty() {
            ui.add_space(8.0);
            let mut clicked_idx = None;
            for (i, hit) in self.mod_search.results.iter().enumerate() {
                let is_selected = self.mod_search.selected == Some(i);
                let card_id = egui::Id::new(("mod_hit", i));
                let pointer_pos = ui.ctx().input(|inp| inp.pointer.hover_pos());

                let frame_response = egui::Frame::none()
                    .fill(theme::Colors::bg_elevated())
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&hit.title)
                                        .size(13.0)
                                        .strong()
                                        .color(theme::Colors::text_primary()),
                                );
                                let desc: String = hit.description.chars().take(60).collect();
                                ui.label(theme::small(&desc));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let dl = format_downloads(hit.downloads);
                                    ui.label(theme::small(&format!("↓ {}", dl)));
                                },
                            );
                        });
                    });

                let card_rect = frame_response.response.rect;
                let is_hovered = pointer_pos.is_some_and(|pos| card_rect.contains(pos));

                if is_selected || is_hovered {
                    let fill = if is_selected {
                        theme::Colors::bg_widget_hover()
                    } else {
                        egui::Color32::from_white_alpha(8)
                    };
                    ui.painter()
                        .rect_filled(card_rect, egui::Rounding::same(6.0), fill);
                }

                let response = ui.interact(card_rect, card_id, egui::Sense::click());
                if is_hovered {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                if response.clicked() {
                    clicked_idx = Some(i);
                }
                ui.add_space(4.0);
            }
            if let Some(i) = clicked_idx {
                self.mod_search.selected = Some(i);
                self.load_mod_versions_from_tab(i);
            }
        }

        if !self.mod_search.versions.is_empty() {
            ui.add_space(8.0);
            let hit_title = self
                .mod_search
                .results
                .get(self.mod_search.selected.unwrap_or(0))
                .map(|h| h.title.as_str())
                .unwrap_or("?");
            ui.label(theme::subheading(&format!(
                "{} '{}'",
                I18n::t(lang, "versions_for"),
                hit_title
            )));
            ui.add_space(6.0);
            let mut do_install = false;
            for ver in self.mod_search.versions.iter().take(10) {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(theme::body(&ver.name));
                        ui.label(theme::small(&format!("[{}]", ver.version_type)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(I18n::t(lang, "install")).clicked() {
                                do_install = true;
                            }
                        });
                    });
                });
                ui.add_space(3.0);
            }
            if do_install {
                self.install_mod_from_tab(instance_dir);
            }
            ui.add_space(6.0);
            if ui.button(I18n::t(lang, "back")).clicked() {
                self.mod_search.versions.clear();
            }
        }
    }

    fn render_cf_results(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        if self.cf_search.searching {
            ui.spinner();
        }

        if let Some(ref pending) = self.pending_cf_install.clone() {
            self.render_cf_install_confirmation(ui, pending);
            return;
        }

        if !self.cf_search.files.is_empty() {
            self.render_cf_file_versions(ui, instance_dir);
            return;
        }

        if !self.cf_search.results.is_empty() {
            ui.add_space(8.0);
            let results = self.cf_search.results.clone();
            let mut clicked_mod: Option<(u32, String)> = None;
            for cf_mod in &results {
                let card_id = egui::Id::new(("cf_mod", cf_mod.id));
                let pointer_pos = ui.ctx().input(|inp| inp.pointer.hover_pos());

                let frame_response = egui::Frame::none()
                    .fill(theme::Colors::bg_elevated())
                    .rounding(egui::Rounding::same(6.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&cf_mod.name)
                                        .size(13.0)
                                        .strong()
                                        .color(theme::Colors::text_primary()),
                                );
                                ui.label(theme::small(&cf_mod.summary));
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let dl = format_downloads(cf_mod.download_count);
                                    ui.label(theme::small(&format!("↓ {}", dl)));
                                },
                            );
                        });
                    });

                let card_rect = frame_response.response.rect;
                let is_hovered = pointer_pos.is_some_and(|pos| card_rect.contains(pos));

                if is_hovered {
                    ui.painter().rect_filled(
                        card_rect,
                        egui::Rounding::same(6.0),
                        egui::Color32::from_white_alpha(8),
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }

                let response = ui.interact(card_rect, card_id, egui::Sense::click());
                if response.clicked() {
                    clicked_mod = Some((cf_mod.id, cf_mod.name.clone()));
                }
                ui.add_space(4.0);
            }
            if let Some((mod_id, mod_name)) = clicked_mod {
                self.cf_search.selected_mod_id = Some(mod_id);
                self.cf_search.selected_mod_name = Some(mod_name);
                self.do_cf_load_files(mod_id);
            }
        }
    }

    fn render_cf_file_versions(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language;
        let mod_name = self
            .cf_search
            .selected_mod_name
            .clone()
            .unwrap_or_else(|| "?".to_string());
        ui.add_space(8.0);
        ui.label(theme::subheading(&format!(
            "{} '{}'",
            I18n::t(lang, "versions_for"),
            mod_name
        )));
        ui.add_space(6.0);

        let files = self.cf_search.files.clone();
        let mut install_mod_id = None;
        for file in files.iter().take(15) {
            let release_label = match file.release_type {
                1 => "[Release]",
                2 => "[Beta]",
                3 => "[Alpha]",
                _ => "[Unknown]",
            };
            theme::list_item_card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(theme::body(&file.display_name));
                    ui.label(theme::small(release_label));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(I18n::t(lang, "install")).clicked() {
                            install_mod_id = self.cf_search.selected_mod_id;
                        }
                    });
                });
            });
            ui.add_space(3.0);
        }

        if let Some(mod_id) = install_mod_id {
            self.do_cf_resolve_deps(mod_id, instance_dir);
        }

        ui.add_space(6.0);
        if ui.button(I18n::t(lang, "back")).clicked() {
            self.cf_search.files.clear();
            self.cf_search.selected_mod_id = None;
            self.cf_search.selected_mod_name = None;
        }
    }

    fn render_cf_install_confirmation(
        &mut self,
        ui: &mut egui::Ui,
        pending: &crate::state::CfPendingInstall,
    ) {
        let lang = self.language;
        let has_deps = pending.deps.iter().any(|d| d.is_dependency);
        let main_dep = pending.deps.iter().find(|d| !d.is_dependency);
        let dep_list: Vec<_> = pending.deps.iter().filter(|d| d.is_dependency).collect();

        egui::Frame::none()
            .fill(theme::Colors::bg_elevated())
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(14.0))
            .stroke(egui::Stroke::new(
                1.5_f32,
                egui::Color32::from_rgb(240, 100, 30),
            ))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(theme::subheading(I18n::t(lang, "confirm_install_cf")));
                ui.add_space(8.0);

                if let Some(main) = main_dep {
                    ui.label(theme::body(&format!(
                        "{}  ({:.1} KB)",
                        main.name,
                        main.file_size as f64 / 1024.0
                    )));
                }

                if has_deps {
                    ui.add_space(8.0);
                    ui.label(theme::muted(&format!(
                        "{} ({}):",
                        I18n::t(lang, "required_deps"),
                        dep_list.len()
                    )));
                    ui.add_space(4.0);
                    for dep in &dep_list {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(theme::small(&format!(
                                "• {}  ({:.1} KB)",
                                dep.name,
                                dep.file_size as f64 / 1024.0
                            )));
                        });
                    }
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(I18n::t(lang, "install")).clicked() {
                        self.do_cf_confirmed_install();
                    }
                    if ui.button(I18n::t(lang, "cancel")).clicked() {
                        self.pending_cf_install = None;
                    }
                });
            });
    }

    fn effective_cf_api_key(&self) -> String {
        self.config
            .curseforge_api_key
            .clone()
            .filter(|k| !k.is_empty())
            .unwrap_or_else(|| crate::app::CF_API_KEY_BUILTIN.to_string())
    }

    fn do_cf_search(&mut self, _instance_dir: &Path) {
        if self.cf_search.query.is_empty() {
            return;
        }
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let api_key = self.effective_cf_api_key();
        self.cf_search.searching = true;
        self.controller.send(AppCommand::CfSearchMods {
            query: self.cf_search.query.clone(),
            mc_version: inst.minecraft_version.clone(),
            loader: inst
                .mod_loader
                .as_ref()
                .map(|l| l.loader_type.as_str().to_string()),
            api_key,
        });
    }

    fn do_cf_load_files(&mut self, mod_id: u32) {
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let api_key = self.effective_cf_api_key();
        self.cf_search.searching = true;
        self.controller.send(AppCommand::CfLoadFiles {
            mod_id,
            mc_version: inst.minecraft_version.clone(),
            loader: inst
                .mod_loader
                .as_ref()
                .map(|l| l.loader_type.as_str().to_string()),
            api_key,
        });
    }

    fn do_cf_resolve_deps(&mut self, mod_id: u32, instance_dir: &Path) {
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let api_key = self.effective_cf_api_key();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string())
            .unwrap_or_else(|| "forge".to_string());
        self.cf_search.searching = true;
        self.controller.send(AppCommand::CfResolveDeps {
            mod_id,
            mc_version: inst.minecraft_version.clone(),
            loader,
            instance_dir: instance_dir.to_path_buf(),
            api_key,
        });
    }

    fn do_cf_confirmed_install(&mut self) {
        let Some(pending) = self.pending_cf_install.take() else {
            return;
        };
        let Some(main_dep) = pending.deps.iter().find(|d| !d.is_dependency) else {
            return;
        };
        self.status = format!("Installing {}...", pending.mod_name);
        self.cf_search.searching = true;
        self.controller.send(AppCommand::CfInstallMod {
            mod_id: main_dep.mod_id,
            mc_version: pending.mc_version,
            loader: pending.loader,
            instance_dir: pending.instance_dir,
            api_key: pending.api_key,
        });
    }

    fn do_mod_search_from_tab(&mut self) {
        if self.mod_search.query.is_empty() {
            return;
        }
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string());
        let query = self.mod_search.query.clone();
        self.mod_search.searching = true;

        self.controller.send(AppCommand::SearchMods {
            query,
            mc_version,
            loader,
        });
    }

    fn load_mod_versions_from_tab(&mut self, hit_idx: usize) {
        let Some(hit) = self.mod_search.results.get(hit_idx) else {
            return;
        };
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let slug = hit.slug.clone();
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string());
        self.mod_search.searching = true;

        self.controller.send(AppCommand::LoadModVersions {
            slug,
            mc_version,
            loader,
        });
    }

    fn render_install_confirmation(
        &mut self,
        ui: &mut egui::Ui,
        pending: &crate::state::PendingModInstall,
    ) {
        let lang = self.language;
        let has_deps = pending.deps.iter().any(|d| d.is_dependency);
        let main_mod = pending.deps.iter().find(|d| !d.is_dependency);
        let dep_mods: Vec<_> = pending.deps.iter().filter(|d| d.is_dependency).collect();

        egui::Frame::none()
            .fill(theme::Colors::bg_elevated())
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(14.0))
            .stroke(egui::Stroke::new(1.5_f32, theme::Colors::accent()))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(theme::subheading(I18n::t(lang, "confirm_install")));
                ui.add_space(8.0);

                if let Some(main) = main_mod {
                    ui.label(theme::body(&format!(
                        "{}  ({:.1} KB)",
                        main.name,
                        main.file_size as f64 / 1024.0
                    )));
                }

                if has_deps {
                    ui.add_space(8.0);
                    ui.label(theme::muted(&format!(
                        "{} ({}):",
                        I18n::t(lang, "required_deps"),
                        dep_mods.len()
                    )));
                    ui.add_space(4.0);
                    for dep in &dep_mods {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(theme::small(&format!(
                                "• {}  ({:.1} KB)",
                                dep.name,
                                dep.file_size as f64 / 1024.0
                            )));
                        });
                    }
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if has_deps {
                        if ui.button(I18n::t(lang, "install_all")).clicked() {
                            self.do_confirmed_install(true);
                        }
                        if ui.button(I18n::t(lang, "only_this_mod")).clicked() {
                            self.do_confirmed_install(false);
                        }
                    } else if ui.button(I18n::t(lang, "install")).clicked() {
                        self.do_confirmed_install(false);
                    }
                    if ui.button(I18n::t(lang, "cancel")).clicked() {
                        self.pending_mod_install = None;
                    }
                });
            });
    }

    fn install_mod_from_tab(&mut self, instance_dir: &Path) {
        let Some(selected) = self.mod_search.selected else {
            return;
        };
        let Some(hit) = self.mod_search.results.get(selected) else {
            return;
        };
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string())
            .unwrap_or_else(|| "fabric".to_string());
        let slug = hit.slug.clone();

        self.status = format!("Resolving dependencies for {}...", hit.title);
        self.mod_search.searching = true;

        self.controller.send(AppCommand::ResolveDeps {
            slug,
            mc_version,
            loader,
            instance_dir: instance_dir.to_path_buf(),
        });
    }

    pub fn do_confirmed_install(&mut self, include_deps: bool) {
        let Some(pending) = self.pending_mod_install.take() else {
            return;
        };

        self.status = format!("Installing {}...", pending.project_slug);
        self.mod_search.searching = false;

        self.controller.send(AppCommand::InstallMod {
            pending,
            include_deps,
        });
    }
}

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
