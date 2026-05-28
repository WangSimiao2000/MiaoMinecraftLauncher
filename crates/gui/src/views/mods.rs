use eframe::egui;
use std::path::Path;

use crate::app::{I18n, MiaoApp};
use crate::messages::AppCommand;
use crate::state::{ModSource, PendingInstall};
use crate::theme;
use crate::widgets::{ModCard, mod_card::ModCardAction};

/// Data needed to render a search result card.
struct SearchResultItem {
    title: String,
    description: String,
    downloads: u64,
}

struct VersionItem {
    name: String,
    release_label: String,
}

struct VersionListAction {
    install: bool,
    back: bool,
}

impl MiaoApp {
    pub fn render_mods_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language.clone();
        self.file_scan_cache.get_or_scan(instance_dir);
        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        let mods = self.file_scan_cache.mods.clone();

        self.handle_file_drop(ui, instance_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!("Mods ({})", mods.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let search_label = if self.mod_search_active {
                    egui::RichText::new(format!(
                        "{} {}",
                        crate::icons::ICON_X_CIRCLE,
                        I18n::t(&lang, "close_search")
                    ))
                    .color(theme::Colors::text_secondary())
                } else {
                    egui::RichText::new(format!(
                        "{} {}",
                        crate::icons::ICON_SEARCH,
                        I18n::t(&lang, "search_mods")
                    ))
                    .color(theme::Colors::text_primary())
                };
                if ui.button(search_label).clicked() {
                    self.mod_search_active = !self.mod_search_active;
                    if self.mod_search_active && !self.mod_search_query.is_empty() {
                        self.do_unified_search(instance_dir);
                    }
                }

                if !self.checking_updates && !mods.is_empty() {
                    if ui.button(I18n::t(&lang, "check_updates")).clicked() {
                        self.check_mod_updates(instance_dir);
                    }
                } else if self.checking_updates {
                    ui.spinner();
                }

                if ui.button(I18n::t(&lang, "open_folder")).clicked() {
                    let _ = miao_core::instance::open_folder(&mods_dir);
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

        if !self.mod_updates.is_empty() {
            ui.add_space(6.0);
            ui.label(theme::subheading(&format!(
                "{} {} {}",
                self.mod_updates.len(),
                I18n::t(&lang, "updates_available"),
                ""
            )));
            ui.add_space(4.0);
            for update in self.mod_updates.clone() {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(theme::body(&update.filename));
                        ui.label(theme::small(&format!("→ {}", update.new_version_number)));
                    });
                });
                ui.add_space(2.0);
            }
            ui.add_space(theme::Spacing::SMALL_GAP);
            ui.separator();
            ui.add_space(theme::Spacing::SMALL_GAP);
        }

        if mods.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted(I18n::t(&lang, "no_mods")));
                ui.add_space(8.0);
                ui.label(theme::small(I18n::t(&lang, "no_mods_hint")));
            });
        } else {
            let row_height = 30.0;
            let total = mods.len();
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .id_salt("installed_mods_scroll")
                .show_rows(ui, row_height, total, |ui, row_range| {
                    for i in row_range {
                        let mut m = mods[i].clone();
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
                });
        }
    }

    fn render_unified_search(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let lang = self.language.clone();
        ui.horizontal(|ui| {
            let mr_selected = self.mod_source == ModSource::Modrinth;
            let cf_selected = self.mod_source == ModSource::CurseForge;

            let mr_btn = if mr_selected {
                egui::Button::new(egui::RichText::new("Modrinth").color(egui::Color32::WHITE))
                    .fill(theme::Colors::accent())
                    .corner_radius(egui::CornerRadius {
                        nw: 6,
                        sw: 6,
                        ne: 0,
                        se: 0,
                    })
            } else {
                egui::Button::new("Modrinth").corner_radius(egui::CornerRadius {
                    nw: 6,
                    sw: 6,
                    ne: 0,
                    se: 0,
                })
            };
            if ui.add(mr_btn).clicked() && !mr_selected {
                self.mod_source = ModSource::Modrinth;
                self.on_source_switched(instance_dir);
            }

            let cf_btn = if cf_selected {
                egui::Button::new(egui::RichText::new("CurseForge").color(egui::Color32::WHITE))
                    .fill(egui::Color32::from_rgb(240, 100, 30))
                    .corner_radius(egui::CornerRadius {
                        nw: 0,
                        sw: 0,
                        ne: 6,
                        se: 6,
                    })
            } else {
                egui::Button::new("CurseForge").corner_radius(egui::CornerRadius {
                    nw: 0,
                    sw: 0,
                    ne: 6,
                    se: 6,
                })
            };
            if ui.add(cf_btn).clicked() && !cf_selected {
                self.mod_source = ModSource::CurseForge;
                self.on_source_switched(instance_dir);
            }
        });

        ui.add_space(6.0);

        // BUG FIX #1: Check for pending install first and render confirmation instead of search results
        if let Some(ref pending) = self.pending_install.clone() {
            self.render_pending_confirmation(ui, pending);
            return;
        }

        if self.mod_source == ModSource::CurseForge && self.effective_cf_api_key().is_empty() {
            ui.label(theme::muted(I18n::t(&lang, "cf_no_key")));
            return;
        }

        ui.horizontal(|ui| {
            ui.label(I18n::t(&lang, "search"));
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.mod_search_query)
                    .desired_width(ui.available_width() - 60.0)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.do_unified_search(instance_dir);
            }
            if ui.button(I18n::t(&lang, "go")).clicked() {
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
                self.mod_search.offset = 0;
                self.mod_search.total_hits = 0;
                self.do_mod_search_from_tab();
            }
            ModSource::CurseForge => {
                self.cf_search.query = self.mod_search_query.clone();
                self.cf_search.active = true;
                self.cf_search.results.clear();
                self.cf_search.offset = 0;
                self.cf_search.total_count = 0;
                self.do_cf_search(instance_dir);
            }
        }
    }

    fn render_modrinth_results(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        if self.mod_search.searching {
            ui.spinner();
        }

        if !self.mod_search.results.is_empty() && self.mod_search.versions.is_empty() {
            ui.add_space(8.0);
            let mut clicked_idx = None;

            let results: Vec<SearchResultItem> = self
                .mod_search
                .results
                .iter()
                .map(|hit| SearchResultItem {
                    title: hit.title.clone(),
                    description: hit.description.chars().take(60).collect(),
                    downloads: hit.downloads,
                })
                .collect();

            if let Some(idx) = self.render_search_results(ui, &results, self.mod_search.selected) {
                clicked_idx = Some(idx);
            }

            if let Some(i) = clicked_idx {
                self.mod_search.selected = Some(i);
                self.load_mod_versions_from_tab(i);
            }

            let has_more =
                self.mod_search.offset < self.mod_search.total_hits && !self.mod_search.searching;
            if has_more {
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    let label = format!(
                        "{} ({}/{})",
                        I18n::t(&self.language, "load_more"),
                        self.mod_search.offset,
                        self.mod_search.total_hits
                    );
                    if ui.button(label).clicked() {
                        self.do_mod_search_from_tab();
                    }
                });
            }
        }

        if !self.mod_search.versions.is_empty() {
            let hit_title = self
                .mod_search
                .results
                .get(self.mod_search.selected.unwrap_or(0))
                .map(|h| h.title.as_str())
                .unwrap_or("?");
            let versions: Vec<VersionItem> = self
                .mod_search
                .versions
                .iter()
                .take(10)
                .map(|ver| VersionItem {
                    name: ver.name.clone(),
                    release_label: format!("[{}]", ver.version_type),
                })
                .collect();
            let action = self.render_version_list(ui, hit_title, &versions);
            if action.install {
                self.install_mod_from_tab(instance_dir);
            }
            if action.back {
                self.mod_search.versions.clear();
            }
        }
    }

    fn render_cf_results(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        if self.cf_search.searching {
            ui.spinner();
        }

        if !self.cf_search.files.is_empty() {
            self.render_cf_file_versions(ui, instance_dir);
            return;
        }

        if !self.cf_search.results.is_empty() {
            ui.add_space(8.0);
            let results: Vec<SearchResultItem> = self
                .cf_search
                .results
                .iter()
                .map(|cf_mod| SearchResultItem {
                    title: cf_mod.name.clone(),
                    description: cf_mod.summary.clone(),
                    downloads: cf_mod.download_count,
                })
                .collect();

            if let Some(idx) = self.render_search_results(ui, &results, None) {
                let cf_mod = &self.cf_search.results[idx];
                self.cf_search.selected_mod_id = Some(cf_mod.id);
                self.cf_search.selected_mod_name = Some(cf_mod.name.clone());
                self.do_cf_load_files(cf_mod.id);
            }

            let has_more =
                self.cf_search.offset < self.cf_search.total_count && !self.cf_search.searching;
            if has_more {
                ui.add_space(6.0);
                ui.vertical_centered(|ui| {
                    let label = format!(
                        "{} ({}/{})",
                        I18n::t(&self.language, "load_more"),
                        self.cf_search.offset,
                        self.cf_search.total_count
                    );
                    if ui.button(label).clicked() {
                        self.do_cf_search(instance_dir);
                    }
                });
            }
        }
    }
    fn render_search_results(
        &mut self,
        ui: &mut egui::Ui,
        results: &[SearchResultItem],
        selected: Option<usize>,
    ) -> Option<usize> {
        let mut clicked_idx = None;

        for (i, item) in results.iter().enumerate() {
            let is_selected = selected == Some(i);
            let card_id = egui::Id::new(("search_result_card", i));

            // BUG FIX #2: Determine fill color BEFORE rendering the Frame
            // Selected state uses bg_widget_hover as the Frame fill (not painted on top)
            // Hover is handled with semi-transparent overlay after rendering
            let fill_color = if is_selected {
                theme::Colors::bg_widget_hover()
            } else {
                theme::Colors::bg_elevated()
            };

            let frame_response = egui::Frame::NONE
                .fill(fill_color)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new(&item.title)
                                    .size(13.0)
                                    .strong()
                                    .color(theme::Colors::text_primary()),
                            );
                            ui.label(theme::small(&item.description));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let dl = format_downloads(item.downloads);
                            ui.label(theme::small(&format!("↓ {}", dl)));
                        });
                    });
                });

            let card_rect = frame_response.response.rect;

            // Check for hover AFTER rendering so we can apply semi-transparent overlay
            let pointer_pos = ui.ctx().input(|inp| inp.pointer.hover_pos());
            let is_hovered = pointer_pos.is_some_and(|pos| card_rect.contains(pos));

            // Apply hover overlay ONLY (semi-transparent, won't hide text)
            // Selected state is already handled via Frame fill
            if is_hovered && !is_selected {
                ui.painter().rect_filled(
                    card_rect,
                    egui::CornerRadius::same(6),
                    theme::Colors::subtle_border(),
                );
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

        clicked_idx
    }

    fn render_version_list(
        &self,
        ui: &mut egui::Ui,
        title: &str,
        versions: &[VersionItem],
    ) -> VersionListAction {
        let lang = self.language.clone();
        let mut action = VersionListAction {
            install: false,
            back: false,
        };

        ui.add_space(8.0);
        ui.label(theme::subheading(&format!(
            "{} '{}'",
            I18n::t(&lang, "versions_for"),
            title
        )));
        ui.add_space(6.0);

        for item in versions {
            theme::list_item_card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(theme::body(&item.name));
                    ui.label(theme::small(&item.release_label));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(I18n::t(&lang, "install")).clicked() {
                            action.install = true;
                        }
                    });
                });
            });
            ui.add_space(3.0);
        }

        ui.add_space(6.0);
        if ui.button(I18n::t(&lang, "back")).clicked() {
            action.back = true;
        }

        action
    }

    fn render_cf_file_versions(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let mod_name = self
            .cf_search
            .selected_mod_name
            .clone()
            .unwrap_or_else(|| "?".to_string());

        let versions: Vec<VersionItem> = self
            .cf_search
            .files
            .iter()
            .take(15)
            .map(|file| VersionItem {
                name: file.display_name.clone(),
                release_label: match file.release_type {
                    1 => "[Release]".to_string(),
                    2 => "[Beta]".to_string(),
                    3 => "[Alpha]".to_string(),
                    _ => "[Unknown]".to_string(),
                },
            })
            .collect();

        let action = self.render_version_list(ui, &mod_name, &versions);
        if action.install
            && let Some(mod_id) = self.cf_search.selected_mod_id
        {
            self.do_cf_resolve_deps(mod_id, instance_dir);
        }
        if action.back {
            self.cf_search.files.clear();
            self.cf_search.selected_mod_id = None;
            self.cf_search.selected_mod_name = None;
        }
    }

    /// Unified confirmation panel for both Modrinth and CurseForge pending installs.
    fn render_pending_confirmation(&mut self, ui: &mut egui::Ui, pending: &PendingInstall) {
        let lang = self.language.clone();

        // Extract common fields based on pending type
        let (
            title_key,
            has_deps,
            main_name,
            main_size_kb,
            dep_count,
            deps_iter,
            stroke_color,
            show_install_all,
        ) = match pending {
            PendingInstall::Modrinth(p) => {
                let has = p.deps.iter().any(|d| d.is_dependency);
                let main = p.deps.iter().find(|d| !d.is_dependency);
                let dep_list: Vec<_> = p.deps.iter().filter(|d| d.is_dependency).collect();
                let count = dep_list.len();
                (
                    "confirm_install",
                    has,
                    main.map(|m| m.name.clone()).unwrap_or_default(),
                    main.map(|m| m.file_size as f64 / 1024.0).unwrap_or(0.0),
                    count,
                    dep_list
                        .into_iter()
                        .map(|d| (d.name.clone(), d.file_size as f64 / 1024.0))
                        .collect::<Vec<_>>(),
                    theme::Colors::accent(),
                    true, // Modrinth shows "Install All" and "Only This Mod"
                )
            }
            PendingInstall::CurseForge(p) => {
                let has = p.deps.iter().any(|d| d.is_dependency);
                let main = p.deps.iter().find(|d| !d.is_dependency);
                let dep_list: Vec<_> = p.deps.iter().filter(|d| d.is_dependency).collect();
                let count = dep_list.len();
                (
                    "confirm_install_cf",
                    has,
                    main.map(|m| m.name.clone())
                        .unwrap_or_else(|| p.mod_name.clone()),
                    main.map(|m| m.file_size as f64 / 1024.0).unwrap_or(0.0),
                    count,
                    dep_list
                        .into_iter()
                        .map(|d| (d.name.clone(), d.file_size as f64 / 1024.0))
                        .collect::<Vec<_>>(),
                    egui::Color32::from_rgb(240, 100, 30),
                    false, // CurseForge shows only "Install"
                )
            }
        };

        egui::Frame::NONE
            .fill(theme::Colors::bg_elevated())
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::same(14))
            .stroke(egui::Stroke::new(1.5_f32, stroke_color))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(theme::subheading(I18n::t(&lang, title_key)));
                ui.add_space(8.0);

                ui.label(theme::body(&format!(
                    "{}  ({:.1} KB)",
                    main_name, main_size_kb
                )));

                if has_deps {
                    ui.add_space(8.0);
                    ui.label(theme::muted(&format!(
                        "{} ({}):",
                        I18n::t(&lang, "required_deps"),
                        dep_count
                    )));
                    ui.add_space(4.0);
                    for (name, size_kb) in &deps_iter {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(theme::small(&format!("• {}  ({:.1} KB)", name, size_kb)));
                        });
                    }
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if show_install_all && has_deps {
                        if ui.button(I18n::t(&lang, "install_all")).clicked() {
                            self.do_confirmed_install(true);
                        }
                        if ui.button(I18n::t(&lang, "only_this_mod")).clicked() {
                            self.do_confirmed_install(false);
                        }
                    } else if ui.button(I18n::t(&lang, "install")).clicked() {
                        match pending {
                            PendingInstall::Modrinth(_) => self.do_confirmed_install(false),
                            PendingInstall::CurseForge(_) => self.do_cf_confirmed_install(),
                        }
                    }
                    if ui.button(I18n::t(&lang, "cancel")).clicked() {
                        self.pending_install = None;
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
            index: self.cf_search.offset,
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
        let Some(pending) = self.pending_install.take() else {
            return;
        };
        let PendingInstall::CurseForge(cf_pending) = pending else {
            return;
        };
        let Some(main_dep) = cf_pending.deps.iter().find(|d| !d.is_dependency) else {
            return;
        };
        self.status = format!("Installing {}...", cf_pending.mod_name);
        self.cf_search.searching = true;
        self.controller.send(AppCommand::CfInstallMod {
            mod_id: main_dep.mod_id,
            mc_version: cf_pending.mc_version,
            loader: cf_pending.loader,
            instance_dir: cf_pending.instance_dir,
            api_key: cf_pending.api_key,
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
            offset: self.mod_search.offset,
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
        let Some(pending) = self.pending_install.take() else {
            return;
        };
        let PendingInstall::Modrinth(mr_pending) = pending else {
            return;
        };

        self.status = format!("Installing {}...", mr_pending.project_slug);
        self.mod_search.searching = false;

        self.controller.send(AppCommand::InstallMod {
            pending: mr_pending,
            include_deps,
        });
    }

    fn check_mod_updates(&mut self, instance_dir: &Path) {
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string())
            .unwrap_or_else(|| "fabric".to_string());
        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        self.checking_updates = true;
        self.mod_updates.clear();
        self.controller.send(AppCommand::CheckModUpdates {
            mods_dir,
            mc_version: inst.minecraft_version.clone(),
            loader,
        });
    }

    fn handle_file_drop(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let dropped = ui.ctx().input(|i| i.raw.dropped_files.clone());
        if dropped.is_empty() {
            return;
        }

        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        let _ = std::fs::create_dir_all(&mods_dir);

        let mut count = 0u32;
        for file in &dropped {
            let Some(path) = &file.path else {
                continue;
            };
            let Some(file_name) = path.file_name() else {
                continue;
            };
            let name = file_name.to_string_lossy();
            if name.ends_with(".jar") || name.ends_with(".zip") {
                let dest = mods_dir.join(file_name);
                if std::fs::copy(path, &dest).is_ok() {
                    count += 1;
                }
            }
        }

        if count > 0 {
            self.file_scan_cache.invalidate();
            self.toasts.success(format!(
                "{} {} {}",
                count,
                I18n::t(&self.language, "files_installed"),
                ""
            ));
        }
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
