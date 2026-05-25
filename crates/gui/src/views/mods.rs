use eframe::egui;
use std::path::Path;

use crate::app::MiaoApp;
use crate::theme;

impl MiaoApp {
    pub fn render_mods_tab(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        let mods = miao_core::modmanager::scan_mods_dir(&mods_dir);

        ui.horizontal(|ui| {
            ui.label(theme::subheading(&format!("Mods ({})", mods.len())));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let search_btn = if self.mod_search.active {
                    egui::Button::new(
                        egui::RichText::new("Search Modrinth").color(egui::Color32::WHITE),
                    )
                    .fill(theme::Colors::ACCENT)
                } else {
                    egui::Button::new("Search Modrinth")
                };
                if ui.add(search_btn).clicked() {
                    if self.mod_search.active {
                        self.mod_search.active = false;
                    } else {
                        self.start_mod_search();
                    }
                }
                if ui.button("Open folder").clicked() {
                    let _ = miao_core::instance::open_folder(&mods_dir);
                }
            });
        });

        ui.add_space(theme::Spacing::SMALL_GAP);

        if self.mod_search.active {
            self.render_inline_mod_search(ui, instance_dir);
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.separator();
            ui.add_space(theme::Spacing::SMALL_GAP);
        }

        let pending = self.async_state.lock().unwrap().pending_mod_install.clone();
        if let Some(ref pending) = pending {
            self.mod_search.active = false;
            self.render_install_confirmation(ui, pending);
            ui.add_space(theme::Spacing::SECTION_GAP);
            ui.separator();
            ui.add_space(theme::Spacing::SMALL_GAP);
        }

        if mods.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::muted("No mods installed"));
                ui.add_space(8.0);
                ui.label(theme::small(
                    "Use 'Search Modrinth' to find and install mods",
                ));
            });
        } else {
            for mut m in mods {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if m.enabled {
                            let btn = egui::Button::new(
                                egui::RichText::new("ON")
                                    .color(theme::Colors::SUCCESS)
                                    .size(12.0),
                            );
                            if ui.add(btn).clicked() {
                                let _ = m.toggle();
                            }
                            ui.label(theme::body(&m.name));
                        } else {
                            let btn = egui::Button::new(
                                egui::RichText::new("OFF")
                                    .color(theme::Colors::TEXT_MUTED)
                                    .size(12.0),
                            );
                            if ui.add(btn).clicked() {
                                let _ = m.toggle();
                            }
                            ui.label(
                                egui::RichText::new(&m.name).color(theme::Colors::TEXT_DISABLED),
                            );
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Del").clicked() {
                                let _ = m.delete();
                            }
                        });
                    });
                });
                ui.add_space(2.0);
            }
        }
    }

    fn start_mod_search(&mut self) {
        self.mod_search.active = true;
        self.mod_search.query.clear();
        self.mod_search.results.clear();
        self.mod_search.versions.clear();
    }

    fn render_inline_mod_search(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.mod_search.query)
                    .vertical_align(egui::Align::Center)
                    .min_size(ui.spacing().interact_size),
            );
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.do_mod_search_from_tab();
            }
            if ui.button("Go").clicked() {
                self.do_mod_search_from_tab();
            }
        });

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
                    .fill(theme::Colors::BG_ELEVATED)
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
                                        .color(theme::Colors::TEXT_PRIMARY),
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
                        theme::Colors::BG_WIDGET_HOVER
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
            ui.label(theme::subheading(&format!("Versions for '{}'", hit_title)));
            ui.add_space(6.0);
            let mut do_install = false;
            for ver in self.mod_search.versions.iter().take(10) {
                theme::list_item_card().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(theme::body(&ver.name));
                        ui.label(theme::small(&format!("[{}]", ver.version_type)));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Install").clicked() {
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
            if ui.button("← Back").clicked() {
                self.mod_search.versions.clear();
            }
        }
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
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        self.mod_search.searching = true;

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            let result = miao_core::modrinth::api::search_mods(
                &http,
                &query,
                Some(&mc_version),
                loader.as_deref(),
                20,
            )
            .await;
            let mut s = state.lock().unwrap();
            match result {
                Ok(r) => {
                    s.mod_search_hits = Some(r.hits);
                }
                Err(e) => {
                    s.install.status = Some(format!("Search failed: {}", e));
                }
            }
            ctx.request_repaint();
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
        let project_slug = hit.slug.clone();
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string());
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        self.mod_search.searching = true;

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            let result = miao_core::modrinth::api::get_project_versions(
                &http,
                &project_slug,
                Some(&mc_version),
                loader.as_deref(),
            )
            .await;
            let mut s = state.lock().unwrap();
            match result {
                Ok(versions) => {
                    s.mod_versions = Some(versions);
                }
                Err(e) => {
                    s.install.status = Some(format!("Failed: {}", e));
                }
            }
            ctx.request_repaint();
        });
    }

    fn render_install_confirmation(
        &mut self,
        ui: &mut egui::Ui,
        pending: &crate::state::PendingModInstall,
    ) {
        let has_deps = pending.deps.iter().any(|d| d.is_dependency);
        let main_mod = pending.deps.iter().find(|d| !d.is_dependency);
        let dep_mods: Vec<_> = pending.deps.iter().filter(|d| d.is_dependency).collect();

        egui::Frame::none()
            .fill(theme::Colors::BG_ELEVATED)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::same(14.0))
            .stroke(egui::Stroke::new(1.5, theme::Colors::ACCENT))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(theme::subheading("Confirm Installation"));
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
                        "Required dependencies ({}):",
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
                        if ui.button("Install All").clicked() {
                            self.do_confirmed_install(true);
                        }
                        if ui.button("Only This Mod").clicked() {
                            self.do_confirmed_install(false);
                        }
                    } else {
                        if ui.button("Install").clicked() {
                            self.do_confirmed_install(false);
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.async_state.lock().unwrap().pending_mod_install = None;
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
        let project_slug = hit.slug.clone();
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        let instance_dir = instance_dir.to_path_buf();

        self.status = format!("Resolving dependencies for {}...", hit.title);
        self.mod_search.searching = true;

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            match miao_core::modrinth::api::resolve_dependencies(
                &http,
                &project_slug,
                &mc_version,
                &loader,
            )
            .await
            {
                Ok(deps) => {
                    let mut s = state.lock().unwrap();
                    s.pending_mod_install = Some(crate::state::PendingModInstall {
                        project_slug,
                        deps,
                        instance_dir,
                        mc_version,
                        loader,
                    });
                }
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.install.status = Some(format!("Resolve failed: {}", e));
                }
            }
            ctx.request_repaint();
        });
    }

    pub fn do_confirmed_install(&mut self, include_deps: bool) {
        let pending = {
            let mut s = self.async_state.lock().unwrap();
            s.pending_mod_install.take()
        };
        let Some(pending) = pending else {
            return;
        };
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        let mods_dir = miao_core::instance::Instance::mods_dir(&pending.instance_dir);

        self.status = format!("Installing {}...", pending.project_slug);
        self.mod_search.active = false;
        self.mod_search.searching = false;

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            let result = if include_deps {
                miao_core::modrinth::api::install_mod_with_dependencies(
                    &http,
                    &pending.project_slug,
                    &pending.mc_version,
                    &pending.loader,
                    &mods_dir,
                )
                .await
            } else {
                miao_core::modrinth::api::install_mod_only(
                    &http,
                    &pending.project_slug,
                    &pending.mc_version,
                    &pending.loader,
                    &mods_dir,
                )
                .await
            };

            let mut s = state.lock().unwrap();
            match result {
                Ok(results) => {
                    s.install.status = Some(format!("Installed {} mod(s)", results.len()));
                }
                Err(e) => {
                    s.install.status = Some(format!("Install failed: {}", e));
                }
            }
            ctx.request_repaint();
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
