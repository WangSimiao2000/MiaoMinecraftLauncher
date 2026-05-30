use eframe::egui;
use std::path::Path;

use miao_core::instance::Instance;
use miao_core::modpack_source::{ResolutionReport, ResolvedStatus};

use crate::app::{I18n, MiaoApp};
use crate::icons::{
    ICON_CHECK_CIRCLE_FILL, ICON_EXCLAMATION_TRIANGLE_FILL, ICON_PAUSE_CIRCLE_FILL,
    ICON_X_CIRCLE_FILL,
};
use crate::theme;

impl MiaoApp {
    pub fn render_modpack_sync_tab(
        &mut self,
        ui: &mut egui::Ui,
        instance_dir: &Path,
        instance: &Instance,
    ) {
        let lang = self.language.clone();
        let Some(sub) = &instance.modpack_subscription else {
            ui.label(theme::muted(I18n::t(&lang, "modpack_sync_no_subscription")));
            return;
        };

        ui.horizontal(|ui| {
            ui.label(theme::subheading(I18n::t(&lang, "modpack_sync_title")));
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        egui::Grid::new("modpack_sync_meta_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_source_id")));
                ui.label(&sub.source_id);
                ui.end_row();

                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_pack_id")));
                ui.label(&sub.pack_id);
                ui.end_row();

                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_pack_version")));
                ui.label(&sub.pack_version);
                ui.end_row();

                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_mc_version")));
                ui.label(&sub.mc_version);
                ui.end_row();

                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_installed_at")));
                ui.label(sub.installed_at.format("%Y-%m-%d %H:%M:%S UTC").to_string());
                ui.end_row();
            });

        ui.add_space(theme::Spacing::SECTION_GAP);
        ui.separator();
        ui.add_space(theme::Spacing::SMALL_GAP);

        let resolved_path = instance_dir.join(".miao-modpack/resolved.json");
        match read_resolved(&resolved_path) {
            Ok(report) => self.render_resolved_mods(ui, &report, &lang),
            Err(e) => {
                let msg = format!(
                    "{}: {e}",
                    I18n::t(&lang, "modpack_sync_resolved_unreadable")
                );
                ui.label(theme::muted(&msg));
            }
        }
    }

    fn render_resolved_mods(&self, ui: &mut egui::Ui, report: &ResolutionReport, lang: &str) {
        ui.label(theme::subheading(I18n::t(lang, "modpack_sync_mod_list")));
        ui.add_space(theme::Spacing::SMALL_GAP);

        let mut counts = StatusCounts::default();
        for m in &report.mods {
            counts.tally(&m.status);
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} {}", ICON_CHECK_CIRCLE_FILL, counts.compatible))
                    .color(theme::Colors::success()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} {}",
                    ICON_EXCLAMATION_TRIANGLE_FILL,
                    counts.pending + counts.ambiguous
                ))
                .color(theme::Colors::warning()),
            );
            ui.label(
                egui::RichText::new(format!("{} {}", ICON_PAUSE_CIRCLE_FILL, counts.deprecated))
                    .color(theme::Colors::text_muted()),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{} {}",
                    ICON_X_CIRCLE_FILL,
                    counts.abandoned + counts.conflict
                ))
                .color(theme::Colors::danger()),
            );
        });
        ui.add_space(theme::Spacing::SMALL_GAP);

        egui::Grid::new("modpack_sync_mods_grid")
            .num_columns(3)
            .spacing([12.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(theme::muted(I18n::t(lang, "modpack_sync_col_mod")));
                ui.label(theme::muted(I18n::t(lang, "modpack_sync_col_status")));
                ui.label(theme::muted(I18n::t(lang, "modpack_sync_col_version")));
                ui.end_row();

                for m in &report.mods {
                    ui.label(&m.display_name);
                    ui.label(status_label(&m.status, lang));
                    ui.label(m.version_id.as_deref().unwrap_or("—"));
                    ui.end_row();
                }
            });
    }
}

fn read_resolved(path: &Path) -> Result<ResolutionReport, String> {
    let raw = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&raw).map_err(|e| e.to_string())
}

#[derive(Default)]
struct StatusCounts {
    compatible: u32,
    pending: u32,
    abandoned: u32,
    conflict: u32,
    deprecated: u32,
    ambiguous: u32,
}

impl StatusCounts {
    fn tally(&mut self, s: &ResolvedStatus) {
        match s {
            ResolvedStatus::Compatible => self.compatible += 1,
            ResolvedStatus::Pending { .. } => self.pending += 1,
            ResolvedStatus::Abandoned { .. } => self.abandoned += 1,
            ResolvedStatus::Conflict { .. } => self.conflict += 1,
            ResolvedStatus::Deprecated { .. } => self.deprecated += 1,
            ResolvedStatus::Ambiguous { .. } => self.ambiguous += 1,
        }
    }
}

fn status_label(s: &ResolvedStatus, lang: &str) -> String {
    let key = match s {
        ResolvedStatus::Compatible => "modpack_status_compatible",
        ResolvedStatus::Pending { .. } => "modpack_status_pending",
        ResolvedStatus::Abandoned { .. } => "modpack_status_abandoned",
        ResolvedStatus::Conflict { .. } => "modpack_status_conflict",
        ResolvedStatus::Deprecated { .. } => "modpack_status_deprecated",
        ResolvedStatus::Ambiguous { .. } => "modpack_status_ambiguous",
    };
    I18n::t(lang, key).to_string()
}
