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
                if ui.button("Search Modrinth").clicked() {
                    self.start_mod_search();
                }
                if ui.button("Open folder").clicked() {
                    let _ = miao_core::instance::open_folder(&mods_dir);
                }
            });
        });

        ui.add_space(theme::Spacing::SMALL_GAP);

        if self.mod_search_active {
            self.render_inline_mod_search(ui, instance_dir);
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
                ui.horizontal(|ui| {
                    if m.enabled {
                        let btn = egui::Button::new(
                            egui::RichText::new("ON").color(theme::Colors::SUCCESS),
                        );
                        if ui.add(btn).clicked() {
                            let _ = m.toggle();
                        }
                        ui.label(theme::body(&m.name));
                    } else {
                        let btn = egui::Button::new(
                            egui::RichText::new("OFF").color(theme::Colors::TEXT_MUTED),
                        );
                        if ui.add(btn).clicked() {
                            let _ = m.toggle();
                        }
                        ui.label(egui::RichText::new(&m.name).color(theme::Colors::TEXT_DISABLED));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("Del").clicked() {
                            let _ = m.delete();
                        }
                    });
                });
            }
        }
    }

    fn start_mod_search(&mut self) {
        self.mod_search_active = true;
        self.mod_search_query.clear();
        self.mod_search_results.clear();
        self.mod_search_versions.clear();
    }

    fn render_inline_mod_search(&mut self, ui: &mut egui::Ui, instance_dir: &Path) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.text_edit_singleline(&mut self.mod_search_query);
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.do_mod_search_from_tab();
            }
            if ui.button("Go").clicked() {
                self.do_mod_search_from_tab();
            }
            if ui.small_button("Close").clicked() {
                self.mod_search_active = false;
            }
        });

        if self.mod_searching {
            ui.spinner();
        }

        if !self.mod_search_results.is_empty() && self.mod_search_versions.is_empty() {
            ui.add_space(4.0);
            let mut clicked_idx = None;
            for (i, hit) in self.mod_search_results.iter().enumerate() {
                let dl = format_downloads(hit.downloads);
                let text = format!("{} ({})", hit.title, dl);
                if ui.link(&text).clicked() {
                    clicked_idx = Some(i);
                }
            }
            if let Some(i) = clicked_idx {
                self.mod_search_selected = i;
                self.load_mod_versions_from_tab(i);
            }
        }

        if !self.mod_search_versions.is_empty() {
            ui.add_space(4.0);
            let hit_title = self
                .mod_search_results
                .get(self.mod_search_selected)
                .map(|h| h.title.as_str())
                .unwrap_or("?");
            ui.label(theme::body(&format!("Versions for '{}':", hit_title)));
            for (i, ver) in self.mod_search_versions.iter().enumerate().take(10) {
                let label = format!("{} [{}]", ver.name, ver.version_type);
                if ui.button(&label).clicked() {
                    self.install_mod_from_tab(instance_dir);
                    break;
                }
                let _ = i;
            }
            if ui.small_button("Back").clicked() {
                self.mod_search_versions.clear();
            }
        }
    }

    fn do_mod_search_from_tab(&mut self) {
        if self.mod_search_query.is_empty() {
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
        let query = self.mod_search_query.clone();
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        self.mod_searching = true;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = miao_core::modrinth::api::search_mods(
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
                        s.install_status = Some(format!("Search failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    fn load_mod_versions_from_tab(&mut self, hit_idx: usize) {
        let Some(hit) = self.mod_search_results.get(hit_idx) else {
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
        self.mod_searching = true;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = miao_core::modrinth::api::get_project_versions(
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
                        s.install_status = Some(format!("Failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    fn install_mod_from_tab(&mut self, instance_dir: &Path) {
        let Some(hit) = self.mod_search_results.get(self.mod_search_selected) else {
            return;
        };
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[idx];
        let mods_dir = miao_core::instance::Instance::mods_dir(instance_dir);
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string())
            .unwrap_or_else(|| "fabric".to_string());
        let project_slug = hit.slug.clone();
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();

        self.status = format!("Installing {} + deps...", hit.title);
        self.mod_search_active = false;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                match miao_core::modrinth::api::install_mod_with_dependencies(
                    &project_slug,
                    &mc_version,
                    &loader,
                    &mods_dir,
                )
                .await
                {
                    Ok(results) => {
                        let mut s = state.lock().unwrap();
                        s.install_status = Some(format!("Installed {} mod(s)", results.len()));
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.install_status = Some(format!("Install failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
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
