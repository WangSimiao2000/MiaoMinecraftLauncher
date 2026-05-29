use eframe::egui;

use miao_core::modpack_source::Loader;

use crate::app::{I18n, MiaoApp};
use crate::messages::AppCommand;
use crate::theme;

impl MiaoApp {
    pub fn render_modpack_tab(&mut self, ui: &mut egui::Ui, lang: &str, open: &mut bool) {
        ui.label(theme::small(I18n::t(lang, "modpack_browse_source")));
        ui.add_space(3.0);

        let source_label = self
            .modpack_source
            .current_source_id
            .clone()
            .unwrap_or_else(|| "miao".to_string());
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("modpack_source_select")
                .selected_text(&source_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.modpack_source.current_source_id,
                        Some("miao".to_string()),
                        "miao",
                    );
                });
            if ui.button(I18n::t(lang, "modpack_browse_refresh")).clicked() {
                self.fetch_modpack_manifest("miao");
            }
        });

        ui.add_space(8.0);

        if self.modpack_source.manifest_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(I18n::t(lang, "modpack_browse_loading"));
            });
            return;
        }
        if let Some(err) = &self.modpack_source.manifest_error {
            ui.label(theme::muted(&format!(
                "{}: {err}",
                I18n::t(lang, "modpack_browse_error")
            )));
            return;
        }

        let Some(manifest) = self.modpack_source.manifest.clone() else {
            ui.label(theme::muted(I18n::t(lang, "modpack_browse_no_manifest")));
            return;
        };

        ui.label(theme::subheading(&manifest.source_name));
        ui.label(theme::small(&format!(
            "{} · {}",
            manifest.author,
            manifest.homepage.as_deref().unwrap_or("")
        )));
        ui.add_space(8.0);

        ui.label(theme::small(I18n::t(lang, "modpack_browse_select_pack")));
        ui.add_space(3.0);

        let mut chosen_idx = self.modpack_source.selected_pack_idx;
        for (idx, pack) in manifest.packs.iter().enumerate() {
            let selected = chosen_idx == Some(idx);
            let label = format!("{} ({:?})", pack.display_name, pack.support_level);
            if ui.selectable_label(selected, label).clicked() {
                chosen_idx = Some(idx);
                self.modpack_source.selected_mc_version = pack.mc_versions.first().cloned();
            }
        }
        self.modpack_source.selected_pack_idx = chosen_idx;

        let Some(idx) = chosen_idx else {
            return;
        };
        let pack = match manifest.packs.get(idx) {
            Some(p) => p.clone(),
            None => return,
        };

        ui.add_space(10.0);
        ui.label(theme::small(I18n::t(lang, "modpack_browse_pack_summary")));
        ui.add_space(3.0);
        ui.label(&pack.summary);

        ui.add_space(10.0);
        ui.label(theme::small(I18n::t(lang, "modpack_browse_mc_version")));
        ui.add_space(3.0);

        let mut selected_mc = self
            .modpack_source
            .selected_mc_version
            .clone()
            .or_else(|| pack.mc_versions.first().cloned())
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("modpack_mc_version")
            .selected_text(&selected_mc)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for v in &pack.mc_versions {
                    ui.selectable_value(&mut selected_mc, v.clone(), v);
                }
            });
        self.modpack_source.selected_mc_version = Some(selected_mc.clone());

        ui.add_space(10.0);
        ui.label(theme::small(I18n::t(lang, "instance_name")));
        ui.add_space(3.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.new_instance.name)
                .desired_width(ui.available_width())
                .hint_text(I18n::t(lang, "instance_name")),
        );

        ui.add_space(theme::Spacing::SECTION_GAP);
        ui.horizontal(|ui| {
            if ui.button(I18n::t(lang, "cancel")).clicked() {
                *open = false;
            }
            if ui.button(I18n::t(lang, "modpack_browse_install")).clicked() {
                if self.new_instance.name.trim().is_empty() {
                    self.toasts.error(I18n::t(lang, "modpack_browse_need_name"));
                } else {
                    let loader_canonical = match pack.loader {
                        Loader::Fabric => "fabric",
                        Loader::Forge => "forge",
                        Loader::Neoforge => "neoforge",
                        Loader::Quilt => "quilt",
                    };
                    self.dispatch_install_modpack(
                        manifest.source_id.clone(),
                        pack.id.clone(),
                        pack.pack_url(&manifest),
                        selected_mc.clone(),
                        loader_canonical.to_string(),
                        self.new_instance.name.clone(),
                    );
                    *open = false;
                }
            }
        });
    }

    pub fn fetch_modpack_manifest(&mut self, source_id: &str) {
        let manifest_url = miao_core::modpack_source::BUILT_IN_SOURCES
            .iter()
            .find(|s| s.source_id == source_id)
            .map(|s| s.manifest_url.to_string());
        let Some(url) = manifest_url else {
            self.modpack_source.manifest_error = Some(format!("unknown source: {source_id}"));
            return;
        };
        self.modpack_source.manifest_loading = true;
        self.modpack_source.manifest_error = None;
        self.controller.send(AppCommand::FetchModpackManifest {
            source_id: source_id.to_string(),
            manifest_url: url,
        });
    }

    pub fn dispatch_install_modpack(
        &mut self,
        source_id: String,
        pack_id: String,
        pack_url: String,
        mc_version: String,
        loader: String,
        instance_name: String,
    ) {
        self.controller.send(AppCommand::InstallModpack {
            source_id,
            pack_id,
            pack_url,
            mc_version,
            loader,
            instance_name,
            config: self.config.clone(),
        });
    }
}

trait PackUrlHelper {
    fn pack_url(&self, manifest: &miao_core::modpack_source::Manifest) -> String;
}

impl PackUrlHelper for miao_core::modpack_source::ManifestPackEntry {
    fn pack_url(&self, _manifest: &miao_core::modpack_source::Manifest) -> String {
        self.pack_url.clone()
    }
}
