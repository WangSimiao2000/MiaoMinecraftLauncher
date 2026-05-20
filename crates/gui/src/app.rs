use eframe::egui;
use miao_core::auth::offline::create_offline_account;
use miao_core::auth::AuthMethod;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};
use miao_core::version::VersionInfo;
use std::sync::{Arc, Mutex};

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
    versions: Arc<Mutex<Vec<VersionInfo>>>,
    active_panel: Panel,
    selected_instance: Option<usize>,
    status: String,
    offline_username_input: String,
    versions_loading: bool,
}

impl MiaoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        let versions: Arc<Mutex<Vec<VersionInfo>>> = Arc::new(Mutex::new(Vec::new()));
        let versions_clone = versions.clone();
        let mirror = config.download_mirror.clone();
        let ctx = cc.egui_ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let http = reqwest::Client::new();
                if let Ok(all_versions) =
                    miao_core::version::manifest::fetch_version_manifest(&http, &mirror).await
                {
                    let releases: Vec<_> = all_versions
                        .into_iter()
                        .filter(|v| v.is_release())
                        .take(50)
                        .collect();
                    *versions_clone.lock().unwrap() = releases;
                    ctx.request_repaint();
                }
            });
        });

        Self {
            config,
            instances,
            versions,
            active_panel: Panel::Instances,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            versions_loading: true,
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
            if ui.button("▶ Launch").clicked()
                && let Some(idx) = self.selected_instance
            {
                self.status = format!("Launching {}...", self.instances[idx].name);
            }
        }
    }

    fn render_versions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Available Versions");
        ui.separator();

        let versions = self.versions.lock().unwrap();
        if versions.is_empty() {
            if self.versions_loading {
                ui.label("Loading versions from server...");
            } else {
                ui.label("No versions available. Check your network.");
            }
        } else {
            self.versions_loading = false;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for ver in versions.iter() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{:<16}", ver.id));
                        ui.label(&ver.release_time);
                    });
                }
            });
        }
    }

    fn render_accounts(&mut self, ui: &mut egui::Ui) {
        ui.heading("Accounts");
        ui.separator();

        if self.config.accounts.is_empty() {
            ui.label("No accounts configured.");
        } else {
            for (i, acc) in self.config.accounts.iter().enumerate() {
                let active = self.config.active_account_index == Some(i);
                let marker = if active { "★" } else { " " };
                let acc_type = if acc.is_microsoft() { "Microsoft" } else { "Offline" };
                ui.label(format!(" {} [{}] {}", marker, acc_type, acc.username()));
            }
        }

        ui.separator();
        ui.heading("Add Offline Account");
        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.offline_username_input);
            if ui.button("Add").clicked() && !self.offline_username_input.is_empty() {
                let account = create_offline_account(&self.offline_username_input);
                self.status = format!("Added: {} ({})", account.username, account.uuid);
                self.config.accounts.push(AuthMethod::Offline(account));
                if self.config.active_account_index.is_none() {
                    self.config.active_account_index = Some(0);
                }
                let _ = self.config.save();
                self.offline_username_input.clear();
            }
        });

        ui.separator();
        if ui.button("Microsoft Login (Device Code)").clicked() {
            self.status = "Microsoft OAuth: open browser to login. (Not yet wired)".to_string();
        }
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
