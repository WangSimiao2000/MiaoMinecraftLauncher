use eframe::egui;

use miao_core::modpack_source::ResolvedStatus;

use crate::app::{I18n, MiaoApp};
use crate::messages::AppCommand;
use crate::theme;

impl MiaoApp {
    pub fn render_modpack_resolving_overlay(&mut self, ctx: &egui::Context) {
        if self.modpack_source.resolving.is_none() {
            return;
        }
        let lang = self.language.clone();

        let frame = egui::Frame::default()
            .fill(theme::Colors::bg_elevated())
            .stroke(egui::Stroke::new(
                1.0,
                theme::Colors::accent().gamma_multiply(0.5),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(20));

        egui::Window::new("modpack_resolving_overlay")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .frame(frame)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add(egui::Spinner::new().size(28.0));
                    ui.add_space(10.0);
                    ui.label(theme::subheading(I18n::t(&lang, "modpack_resolving_title")));
                    ui.add_space(6.0);
                    ui.label(theme::muted(I18n::t(&lang, "modpack_resolving_hint")));
                });
            });
        ctx.request_repaint();
    }

    pub fn render_modpack_confirm_dialog(&mut self, ctx: &egui::Context) {
        if self.modpack_source.pending_confirm.is_none() {
            return;
        }
        let lang = self.language.clone();

        let frame = egui::Frame::default()
            .fill(theme::Colors::bg_elevated())
            .stroke(egui::Stroke::new(
                1.0,
                theme::Colors::accent().gamma_multiply(0.5),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::same(16));

        let mut continue_clicked = false;
        let mut cancel_clicked = false;
        let mut reresolve_clicked = false;

        egui::Window::new(I18n::t(&lang, "modpack_confirm_title"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .default_width(480.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .frame(frame)
            .show(ctx, |ui| {
                ui.label(theme::subheading(I18n::t(&lang, "modpack_confirm_title")));
                ui.add_space(6.0);
                ui.label(theme::muted(I18n::t(&lang, "modpack_confirm_summary")));
                ui.add_space(10.0);

                let pending = self.modpack_source.pending_confirm.as_ref().unwrap();
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        egui::Grid::new("modpack_confirm_grid")
                            .num_columns(2)
                            .spacing([16.0, 4.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_col_mod")));
                                ui.label(theme::muted(I18n::t(&lang, "modpack_sync_col_status")));
                                ui.end_row();

                                for m in &pending.report.mods {
                                    ui.label(&m.display_name);
                                    let (label_key, color) = status_style(&m.status);
                                    let desc_key = status_desc_key(&m.status);
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(I18n::t(&lang, label_key))
                                                .color(color),
                                        )
                                        .sense(egui::Sense::hover()),
                                    )
                                    .on_hover_text(I18n::t(&lang, desc_key));
                                    ui.end_row();
                                }
                            });
                    });

                ui.add_space(8.0);
                ui.label(theme::muted(I18n::t(&lang, "modpack_confirm_legend_hint")));

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(I18n::t(&lang, "modpack_confirm_continue"))
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(theme::Colors::accent()),
                            )
                            .clicked()
                        {
                            continue_clicked = true;
                        }
                        ui.add_space(8.0);
                        if ui
                            .button(I18n::t(&lang, "modpack_confirm_cancel"))
                            .clicked()
                        {
                            cancel_clicked = true;
                        }
                        ui.add_space(8.0);
                        if ui
                            .button(I18n::t(&lang, "modpack_confirm_reresolve"))
                            .on_hover_text(I18n::t(&lang, "modpack_confirm_reresolve_tip"))
                            .clicked()
                        {
                            reresolve_clicked = true;
                        }
                    });
                });
            });

        if continue_clicked {
            if let Some(p) = self.modpack_source.pending_confirm.take() {
                self.controller.send(AppCommand::ApplyModpackInstall {
                    source_id: p.source_id,
                    pack_id: p.pack_id,
                    pack_url: p.pack_url,
                    instance_name: p.instance_name,
                    config: p.config,
                    report: p.report,
                    pack_raw: p.pack_raw,
                    pack: p.pack,
                });
            }
        } else if cancel_clicked {
            self.modpack_source.pending_confirm = None;
            self.status = "Cancelled.".to_string();
        } else if reresolve_clicked && let Some(p) = self.modpack_source.pending_confirm.take() {
            let loader = match p.pack.loader {
                miao_core::modpack_source::Loader::Fabric => "fabric",
                miao_core::modpack_source::Loader::Forge => "forge",
                miao_core::modpack_source::Loader::Neoforge => "neoforge",
                miao_core::modpack_source::Loader::Quilt => "quilt",
            };
            self.modpack_source.resolving = Some(p.instance_name.clone());
            self.controller.send(AppCommand::ResolveModpack {
                source_id: p.source_id,
                pack_id: p.pack_id,
                pack_url: p.pack_url,
                mc_version: p.report.mc_version.clone(),
                loader: loader.to_string(),
                instance_name: p.instance_name,
                config: *p.config,
            });
        }
    }
}

fn status_style(s: &ResolvedStatus) -> (&'static str, egui::Color32) {
    match s {
        ResolvedStatus::Compatible => ("modpack_status_compatible", theme::Colors::success()),
        ResolvedStatus::Pending { .. } => ("modpack_status_pending", theme::Colors::warning()),
        ResolvedStatus::Abandoned { .. } => ("modpack_status_abandoned", theme::Colors::danger()),
        ResolvedStatus::Conflict { .. } => ("modpack_status_conflict", theme::Colors::danger()),
        ResolvedStatus::Deprecated { .. } => {
            ("modpack_status_deprecated", theme::Colors::text_muted())
        }
        ResolvedStatus::Ambiguous { .. } => ("modpack_status_ambiguous", theme::Colors::warning()),
    }
}

fn status_desc_key(s: &ResolvedStatus) -> &'static str {
    match s {
        ResolvedStatus::Compatible => "modpack_status_compatible_desc",
        ResolvedStatus::Pending { .. } => "modpack_status_pending_desc",
        ResolvedStatus::Abandoned { .. } => "modpack_status_abandoned_desc",
        ResolvedStatus::Conflict { .. } => "modpack_status_conflict_desc",
        ResolvedStatus::Deprecated { .. } => "modpack_status_deprecated_desc",
        ResolvedStatus::Ambiguous { .. } => "modpack_status_ambiguous_desc",
    }
}
