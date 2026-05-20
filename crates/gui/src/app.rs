use eframe::egui;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Instances,
    Versions,
    Accounts,
    Settings,
}

pub struct MiaoApp {
    config: LauncherConfig,
    instances: Vec<Instance>,
    active_panel: Panel,
    selected_instance: Option<usize>,
    status: String,
}

impl MiaoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        Self {
            config,
            instances,
            active_panel: Panel::Instances,
            selected_instance: None,
            status: "Ready".to_string(),
        }
    }
}

impl eframe::App for MiaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🐱 MiaoMC");
                ui.separator();
                ui.selectable_value(&mut self.active_panel, Panel::Instances, "Instances");
                ui.selectable_value(&mut self.active_panel, Panel::Versions, "Versions");
                ui.selectable_value(&mut self.active_panel, Panel::Accounts, "Accounts");
                ui.selectable_value(&mut self.active_panel, Panel::Settings, "Settings");
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.active_panel {
            Panel::Instances => self.render_instances(ui),
            Panel::Versions => self.render_versions(ui),
            Panel::Accounts => self.render_accounts(ui),
            Panel::Settings => self.render_settings(ui),
        });
    }
}

impl MiaoApp {
    fn render_instances(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game Instances");
        ui.separator();

        if self.instances.is_empty() {
            ui.label("No instances yet. Click 'New Instance' to create one.");
            if ui.button("New Instance").clicked() {
                self.status = "Instance creation not yet implemented.".to_string();
            }
        } else {
            for (i, inst) in self.instances.iter().enumerate() {
                let selected = self.selected_instance == Some(i);
                let label = format!(
                    "{} - MC {}{}",
                    inst.name,
                    inst.minecraft_version,
                    inst.mod_loader
                        .as_ref()
                        .map(|l| format!(" [{}]", l.loader_type))
                        .unwrap_or_default()
                );

                if ui.selectable_label(selected, &label).clicked() {
                    self.selected_instance = Some(i);
                    self.status = format!("Selected: {}", inst.name);
                }
            }

            ui.separator();
            if ui.button("▶ Launch").clicked() {
                if let Some(idx) = self.selected_instance {
                    self.status = format!("Launching {}...", self.instances[idx].name);
                }
            }
        }
    }

    fn render_versions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Available Versions");
        ui.separator();
        ui.label("Version list loading not yet implemented.");
        ui.label("Will show releases, snapshots, and mod loader versions.");
    }

    fn render_accounts(&mut self, ui: &mut egui::Ui) {
        ui.heading("Accounts");
        ui.separator();

        if self.config.accounts.is_empty() {
            ui.label("No accounts configured.");
        } else {
            for (i, acc) in self.config.accounts.iter().enumerate() {
                let active = self.config.active_account_index == Some(i);
                let prefix = if active { "★ " } else { "  " };
                let acc_type = if acc.is_microsoft() { "Microsoft" } else { "Offline" };
                ui.label(format!("{}{} ({})", prefix, acc.username(), acc_type));
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Add Microsoft Account").clicked() {
                self.status = "Microsoft login not yet implemented.".to_string();
            }
            if ui.button("Add Offline Account").clicked() {
                self.status = "Offline account creation not yet implemented.".to_string();
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();

        ui.label(format!("Data directory: {}", self.config.data_dir.display()));
        ui.label(format!(
            "Download mirror: {:?}",
            self.config.download_mirror
        ));
        ui.label(format!(
            "Max concurrent downloads: {}",
            self.config.max_concurrent_downloads
        ));

        ui.separator();
        ui.label("Java installations:");
        if self.config.java_paths.is_empty() {
            ui.label("  None configured (will auto-detect)");
        } else {
            for path in &self.config.java_paths {
                ui.label(format!("  {}", path.display()));
            }
        }
    }
}
