use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::version::VersionInfo;
use tokio::runtime::Runtime;

use crate::controller::AppController;
use crate::messages::{AppCommand, AppEvent};
pub use crate::state::{
    AppView, AuthUiState, DetailTab, Dialog, GameLogState, I18n, InstallProgress,
    InstanceSettingsEdit, Language, LoaderUiState, ModSearchState, NewInstanceInput,
    PendingModInstall, SettingsTab, VersionsUiState,
};
use crate::theme;

pub use miao_core::auth::MS_CLIENT_ID;

pub struct MiaoApp {
    pub config: LauncherConfig,
    pub instances: Vec<Instance>,

    pub app_view: AppView,
    pub active_dialog: Dialog,
    pub active_tab: DetailTab,
    pub selected_instance: Option<usize>,
    pub status: String,

    pub offline_username_input: String,
    pub data_dir_input: String,
    pub new_instance: NewInstanceInput,
    pub mod_search: ModSearchState,
    pub pending_mod_install: Option<PendingModInstall>,

    pub confirm_delete: Option<usize>,
    pub settings_tab: SettingsTab,
    pub cached_javas: Option<Vec<miao_core::java::JavaInstallation>>,
    pub refresh_counter: u32,
    #[allow(dead_code)]
    pub theme_preset: crate::theme::ThemePreset,

    pub instance_settings_edit: InstanceSettingsEdit,
    pub language: Language,
    pub mirror_custom_url: String,
    pub max_downloads_input: String,

    pub versions: VersionsUiState,
    pub loader: LoaderUiState,
    pub auth: AuthUiState,
    pub game_log: GameLogState,
    pub install_progress: std::collections::HashMap<String, InstallProgress>,
    pub active_installs: std::collections::HashSet<String>,
    pub update_available: Option<String>,

    pub file_scan_cache: FileScanCache,

    pub controller: AppController,
    #[allow(dead_code)]
    pub rt: Arc<Runtime>,
}

pub struct FileScanCache {
    pub mods: Vec<miao_core::modmanager::ModInfo>,
    pub resourcepacks: Vec<miao_core::resource::ResourcePack>,
    pub shaderpacks: Vec<miao_core::resource::ShaderPack>,
    pub saves: Vec<miao_core::instance::SaveWorld>,
    pub instance_dir: PathBuf,
    pub last_scan: Instant,
}

impl FileScanCache {
    const CACHE_DURATION_MS: u128 = 2000;

    pub fn new() -> Self {
        Self {
            mods: Vec::new(),
            resourcepacks: Vec::new(),
            shaderpacks: Vec::new(),
            saves: Vec::new(),
            instance_dir: PathBuf::new(),
            last_scan: Instant::now(),
        }
    }

    pub fn get_or_scan(&mut self, instance_dir: &std::path::Path) -> bool {
        let now = Instant::now();
        let stale = now.duration_since(self.last_scan).as_millis() > Self::CACHE_DURATION_MS;
        let dir_changed = self.instance_dir != instance_dir;

        if stale || dir_changed {
            self.instance_dir = instance_dir.to_path_buf();
            self.mods = miao_core::modmanager::scan_mods_dir(
                &miao_core::instance::Instance::mods_dir(instance_dir),
            );
            self.resourcepacks = miao_core::resource::scan_resourcepacks(
                &miao_core::instance::Instance::resourcepacks_dir(instance_dir),
            );
            self.shaderpacks = miao_core::resource::scan_shaderpacks(
                &miao_core::instance::Instance::shaderpacks_dir(instance_dir),
            );
            self.saves = miao_core::instance::list_saves(instance_dir);
            self.last_scan = now;
            true
        } else {
            false
        }
    }

    pub fn invalidate(&mut self) {
        self.last_scan = Instant::now() - std::time::Duration::from_secs(10);
    }
}

impl MiaoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        let rt = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
        let controller = AppController::new(&rt, cc.egui_ctx.clone());

        controller.send(AppCommand::FetchVersionManifest {
            mirror: config.download_mirror.clone(),
        });
        controller.send(AppCommand::CheckForUpdates);

        let data_dir_input = config.data_dir.display().to_string();
        let mirror_custom_url = match &config.download_mirror {
            miao_core::config::DownloadMirror::Custom(url) => url.clone(),
            _ => String::new(),
        };
        let max_downloads_input = config.max_concurrent_downloads.to_string();

        Self {
            config,
            instances,
            app_view: AppView::Main,
            active_dialog: Dialog::None,
            active_tab: DetailTab::Mods,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            data_dir_input,
            new_instance: NewInstanceInput::default(),
            mod_search: ModSearchState::default(),
            pending_mod_install: None,
            confirm_delete: None,
            settings_tab: SettingsTab::default(),
            cached_javas: None,
            refresh_counter: 0,
            theme_preset: crate::theme::ThemePreset::Dark,
            instance_settings_edit: InstanceSettingsEdit::default(),
            language: Language::default(),
            mirror_custom_url,
            max_downloads_input,
            versions: VersionsUiState {
                loading: true,
                ..Default::default()
            },
            loader: LoaderUiState::default(),
            auth: AuthUiState::default(),
            game_log: GameLogState::default(),
            install_progress: std::collections::HashMap::new(),
            active_installs: std::collections::HashSet::new(),
            update_available: None,
            file_scan_cache: FileScanCache::new(),
            controller,
            rt,
        }
    }

    pub fn open_new_instance_dialog(&mut self) {
        self.active_dialog = Dialog::NewInstance;
        self.new_instance.reset();
    }

    fn drain_events(&mut self) {
        while let Some(event) = self.controller.try_recv() {
            match event {
                AppEvent::VersionsFetched {
                    all_versions,
                    releases,
                } => {
                    self.versions.all_versions = all_versions;
                    self.versions.versions = releases;
                    self.versions.loading = false;
                }
                AppEvent::LoaderVersionsFetched { versions } => {
                    self.loader.versions = versions;
                    self.loader.loading = false;
                }
                AppEvent::LoaderFetchFailed => {
                    self.loader.loading = false;
                }
                AppEvent::InstallStatus(msg) => {
                    self.status = msg;
                }
                AppEvent::InstallProgress {
                    task_id,
                    completed,
                    total,
                    label,
                } => {
                    self.active_installs.insert(task_id.clone());
                    self.install_progress.insert(task_id, InstallProgress {
                        completed,
                        total,
                        label,
                    });
                }
                AppEvent::InstallFinished { task_id, success, message } => {
                    self.active_installs.remove(&task_id);
                    self.install_progress.remove(&task_id);
                    self.status = message;
                    if success {
                        self.instances = instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                        self.cached_javas = None;
                    }
                }
                AppEvent::DeviceCode(dc) => {
                    self.auth.device_code = Some(dc);
                }
                AppEvent::LoginComplete { account } => {
                    if let AuthMethod::Microsoft(ref ms) = account {
                        self.status = format!("✓ Logged in as {}", ms.username);
                    }
                    self.config.accounts.push(account);
                    if self.config.active_account_index.is_none() {
                        self.config.active_account_index = Some(0);
                    }
                    let _ = self.config.save();
                    self.auth.device_code = None;
                    self.auth.logging_in = false;
                }
                AppEvent::LoginFailed(msg) => {
                    self.status = msg;
                    self.auth.device_code = None;
                    self.auth.logging_in = false;
                }
                AppEvent::JavaProgress(msg) => {
                    self.status = msg;
                }
                AppEvent::JavaInstalled { launch_idx } => {
                    self.cached_javas = None;
                    self.status = "✓ Java installed — launching game...".to_string();
                    self.launch_instance(launch_idx);
                }
                AppEvent::JavaFailed(msg) => {
                    self.status = format!("✗ {}", msg);
                }
                AppEvent::ModSearchResults(hits) => {
                    self.mod_search.results = hits;
                    self.mod_search.searching = false;
                }
                AppEvent::ModVersions(versions) => {
                    self.mod_search.versions = versions;
                    self.mod_search.searching = false;
                }
                AppEvent::ModPendingInstall(pending) => {
                    self.pending_mod_install = Some(pending);
                    self.mod_search.searching = false;
                }
                AppEvent::ModInstalled { count } => {
                    self.status = format!("Installed {} mod(s)", count);
                    self.mod_search.searching = false;
                    self.file_scan_cache.invalidate();
                }
                AppEvent::ModError(msg) => {
                    self.status = msg;
                    self.mod_search.searching = false;
                }
                AppEvent::GameLogLine(line) => {
                    if self.game_log.lines.len() >= GameLogState::MAX_LINES {
                        self.game_log.lines.pop_front();
                    }
                    self.game_log.lines.push_back(line);
                    self.game_log.running = true;
                }
                AppEvent::GameExited => {
                    self.game_log.running = false;
                }
                AppEvent::ExportResult(msg) => {
                    self.status = msg;
                }
                AppEvent::ImportResult { success, message } => {
                    self.status = message;
                    if success {
                        self.instances = instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                    }
                }
                AppEvent::UpdateAvailable(version) => {
                    self.update_available = Some(version);
                }
                AppEvent::TaskCancelled { task_id } => {
                    self.active_installs.remove(&task_id);
                    self.install_progress.remove(&task_id);
                    self.status = "Cancelled.".to_string();
                }
                AppEvent::Error(msg) => {
                    self.status = msg;
                }
            }
        }
    }
}

impl eframe::App for MiaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_events();

        self.refresh_counter += 1;
        if self.refresh_counter.is_multiple_of(60) {
            let fresh = instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
            if fresh.len() != self.instances.len() {
                self.instances = fresh;
                if let Some(idx) = self.selected_instance
                    && idx >= self.instances.len()
                {
                    self.selected_instance = None;
                }
            }
        }

        if !self.active_installs.is_empty()
            || self.auth.logging_in
            || self.loader.loading
            || self.mod_search.searching
            || self.game_log.running
        {
            ctx.request_repaint();
        }

        let lang = self.language;

        egui::TopBottomPanel::bottom("status_bar")
            .frame(theme::bottom_bar_frame())
            .show(ctx, |ui| {
                let active_progress = self.install_progress.values().next();
                if let Some(progress) = active_progress {
                    if progress.total > 0 {
                        let fraction = progress.completed as f32 / progress.total.max(1) as f32;
                        let task_count = self.active_installs.len();
                        let text = if task_count > 1 {
                            format!(
                                "{} ({}/{}) [+{} tasks]",
                                progress.label, progress.completed, progress.total, task_count - 1
                            )
                        } else {
                            format!(
                                "{} ({}/{})",
                                progress.label, progress.completed, progress.total
                            )
                        };
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::ProgressBar::new(fraction)
                                    .text(text)
                                    .fill(theme::Colors::ACCENT),
                            );
                            if ui.small_button("✕").clicked() {
                                self.cancel_all_tasks();
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(theme::status_text(&self.status));
                            if ui.small_button("✕").clicked() {
                                self.cancel_all_tasks();
                            }
                        });
                    }
                } else if !self.active_installs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(theme::status_text(&self.status));
                        if ui.small_button("✕").clicked() {
                            self.cancel_all_tasks();
                        }
                    });
                } else {
                    ui.label(theme::status_text(&self.status));
                }
            });

        match self.app_view {
            AppView::Settings => {
                egui::TopBottomPanel::top("top_bar")
                    .frame(theme::top_bar_frame())
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button(I18n::t(lang, "back")).clicked() {
                                self.app_view = AppView::Main;
                            }
                            ui.add_space(8.0);
                            ui.label(theme::heading(I18n::t(lang, "settings")));
                        });
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_settings_page(ui);
                });
            }
            AppView::Main => {
                egui::TopBottomPanel::top("top_bar")
                    .frame(theme::top_bar_frame())
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(theme::heading("MMCL"));
                            ui.add_space(8.0);
                            ui.label(theme::small("MiaoMinecraftLauncher"));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button(I18n::t(lang, "settings")).clicked() {
                                        self.app_view = AppView::Settings;
                                    }
                                },
                            );
                        });
                    });

                self.render_sidebar(ctx);

                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::none()
                            .fill(theme::Colors::BG_MAIN)
                            .inner_margin(egui::Margin::same(12.0)),
                    )
                    .show(ctx, |ui| {
                        self.render_detail(ui);
                    });

                match self.active_dialog {
                    Dialog::NewInstance => self.render_new_instance_dialog(ctx),
                    Dialog::ConfirmJavaDownload {
                        instance_idx,
                        java_major,
                    } => {
                        self.render_java_download_confirm(ctx, instance_idx, java_major);
                    }
                    Dialog::None => {}
                }
            }
        }
    }
}

impl MiaoApp {
    pub fn launch_instance(&mut self, idx: usize) {
        let inst = &self.instances[idx];

        let java_installations = java::detect_java_with_data_dir(&self.config.data_dir);
        let meta_path = self
            .config
            .versions_dir()
            .join(&inst.minecraft_version)
            .join(format!("{}.json", inst.minecraft_version));

        if !meta_path.exists() {
            self.status = format!("Version meta not found for {}.", inst.minecraft_version);
            return;
        }

        let meta_content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(e) => {
                self.status = format!("Error: {}", e);
                return;
            }
        };

        let meta: miao_core::version::meta::VersionMeta = match serde_json::from_str(&meta_content)
        {
            Ok(m) => m,
            Err(e) => {
                self.status = format!("Parse error: {}", e);
                return;
            }
        };

        let required_java = meta.required_java_major();
        let java_path = inst.java_path.clone().or_else(|| {
            java::find_compatible_java(&java_installations, required_java).map(|j| j.path.clone())
        });

        let Some(_java_path) = java_path else {
            self.status = format!("Java {} not found. Confirm download?", required_java);
            self.active_dialog = Dialog::ConfirmJavaDownload {
                instance_idx: idx,
                java_major: required_java,
            };
            return;
        };

        self.game_log.lines.clear();
        self.game_log.running = true;
        self.status = format!("Launched {}", inst.name);

        self.controller.send(AppCommand::LaunchInstance {
            idx,
            instance: inst.clone(),
            config: self.config.clone(),
        });
    }

    pub fn download_java_for_instance(&mut self, idx: usize) {
        let inst = &self.instances[idx];
        let meta_path = self
            .config
            .versions_dir()
            .join(&inst.minecraft_version)
            .join(format!("{}.json", inst.minecraft_version));

        if !meta_path.exists() {
            self.status = "Version meta not found.".to_string();
            return;
        }

        let Ok(meta_content) = std::fs::read_to_string(&meta_path) else {
            self.status = "Cannot read version meta.".to_string();
            return;
        };

        let Ok(meta) = serde_json::from_str::<miao_core::version::meta::VersionMeta>(&meta_content)
        else {
            self.status = "Cannot parse version meta.".to_string();
            return;
        };

        let required = meta.required_java_major();
        if java::find_compatible_java(
            &java::detect_java_with_data_dir(&self.config.data_dir),
            required,
        )
        .is_some()
        {
            self.status = format!("Java {} already available.", required);
            return;
        }

        let task_id = format!("java-{}", idx);
        if self.active_installs.contains(&task_id) {
            self.status = "Already downloading Java...".to_string();
            return;
        }
        self.active_installs.insert(task_id.clone());
        self.status = format!("Downloading Java {}...", required);

        self.controller.send(AppCommand::DownloadJava {
            task_id,
            required_major: required,
            java_dir: self.config.data_dir.join("java"),
            launch_idx: idx,
        });
    }

    pub fn create_instance(
        &mut self,
        ver: VersionInfo,
        name: String,
        loader: Option<(String, String)>,
    ) {
        let task_id = format!("install-{}", name);
        if self.active_installs.contains(&task_id) {
            self.status = format!("'{}' is already being created.", name);
            return;
        }
        self.active_installs.insert(task_id.clone());
        self.status = format!("Creating '{}'...", name);

        self.controller.send(AppCommand::CreateInstance {
            task_id,
            ver,
            name,
            loader,
            config: self.config.clone(),
        });
    }

    pub fn start_ms_login(&mut self) {
        if self.auth.logging_in {
            return;
        }
        self.auth.logging_in = true;

        self.controller.send(AppCommand::StartMsLogin {
            client_id: MS_CLIENT_ID.to_string(),
            config: self.config.clone(),
        });
    }

    pub fn export_instance_with_dialog(&mut self, idx: usize) {
        let inst = self.instances[idx].clone();
        self.status = "Selecting export folder...".to_string();

        self.controller.send(AppCommand::ExportInstance {
            instance: inst,
            config: self.config.clone(),
        });
    }

    pub fn import_with_dialog(&mut self) {
        let task_id = "import-mrpack".to_string();
        self.status = "Selecting .mrpack file...".to_string();
        self.active_installs.insert(task_id.clone());

        self.controller.send(AppCommand::ImportMrpack {
            task_id,
            config: self.config.clone(),
        });
    }

    pub fn cancel_all_tasks(&mut self) {
        let task_ids: Vec<String> = self.active_installs.iter().cloned().collect();
        for task_id in task_ids {
            self.controller.send(AppCommand::CancelTask { task_id });
        }
    }

    pub fn fetch_loader_versions(&mut self, mc_version: &str) {
        if self.loader.loading {
            return;
        }
        self.loader.loading = true;
        self.loader.versions.clear();

        self.controller.send(AppCommand::FetchLoaderVersions {
            mc_version: mc_version.to_string(),
        });
    }

    pub fn get_available_loaders(&self) -> Vec<(usize, &'static str, bool)> {
        let mut loaders = vec![(0, "None (Vanilla)", true)];
        for (idx, name, loader_type) in [
            (1, "Fabric", ModLoaderType::Fabric),
            (2, "Quilt", ModLoaderType::Quilt),
            (3, "NeoForge", ModLoaderType::NeoForge),
            (4, "Forge", ModLoaderType::Forge),
        ] {
            loaders.push((idx, name, self.loader.versions.contains_key(&loader_type)));
        }
        loaders
    }

    pub fn get_loader_versions(&self) -> Vec<&ModLoaderVersion> {
        if self.new_instance.loader == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.new_instance.loader - 1)
            .and_then(|lt| self.loader.versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn load_instance_settings_edit(&mut self, idx: usize) {
        let inst = &self.instances[idx];
        self.instance_settings_edit = InstanceSettingsEdit {
            memory_max: inst.memory_max_mb.to_string(),
            memory_min: inst.memory_min_mb.to_string(),
            resolution_width: inst
                .resolution
                .as_ref()
                .map(|r| r.width.to_string())
                .unwrap_or_default(),
            resolution_height: inst
                .resolution
                .as_ref()
                .map(|r| r.height.to_string())
                .unwrap_or_default(),
            java_path: inst
                .java_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            jvm_args: inst.jvm_args.join(" "),
            dirty: false,
        };
    }

    pub fn save_instance_settings(&mut self) {
        let Some(idx) = self.selected_instance else {
            return;
        };
        let inst = &mut self.instances[idx];

        if let Ok(v) = self.instance_settings_edit.memory_max.parse::<u32>() {
            inst.memory_max_mb = v;
        }
        if let Ok(v) = self.instance_settings_edit.memory_min.parse::<u32>() {
            inst.memory_min_mb = v;
        }

        let w = self
            .instance_settings_edit
            .resolution_width
            .parse::<u32>()
            .ok();
        let h = self
            .instance_settings_edit
            .resolution_height
            .parse::<u32>()
            .ok();
        inst.resolution = match (w, h) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some(instance::Resolution {
                width: w,
                height: h,
            }),
            _ => None,
        };

        let jp = &self.instance_settings_edit.java_path;
        inst.java_path = if jp.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(jp))
        };

        let args_str = &self.instance_settings_edit.jvm_args;
        inst.jvm_args = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str.split_whitespace().map(|s| s.to_string()).collect()
        };

        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let _ = inst.save_to(&instance_dir);
        self.instance_settings_edit.dirty = false;
        self.status = I18n::t(self.language, "saved").to_string();
    }

    pub fn update_version_filter(
        &mut self,
        show_snapshots: bool,
        show_old_beta: bool,
        show_old_alpha: bool,
    ) {
        use miao_core::version::VersionType;

        self.versions.show_snapshots = show_snapshots;
        self.versions.show_old_beta = show_old_beta;
        self.versions.show_old_alpha = show_old_alpha;

        let filtered: Vec<_> = self
            .versions
            .all_versions
            .iter()
            .filter(|v| match v.version_type {
                VersionType::Release => true,
                VersionType::Snapshot => show_snapshots,
                VersionType::OldBeta => show_old_beta,
                VersionType::OldAlpha => show_old_alpha,
            })
            .cloned()
            .collect();

        self.versions.versions = filtered;
        self.new_instance.version_idx = 0;
        self.new_instance.loader = 0;
        self.new_instance.loader_version_idx = 0;
    }
}
