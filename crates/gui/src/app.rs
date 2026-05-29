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

use std::sync::Mutex;

use crate::animation::{PRESET_BOUNCY, Spring};
use crate::blur::BlurRenderer;
use crate::controller::AppController;
pub use crate::dialogs::setup_wizard::SetupStep;
use crate::messages::{AppCommand, AppEvent};
use crate::navigation::{NavigationStack, Page};
use crate::platform;
pub use crate::state::{
    AuthUiState, CfPendingInstall, CfSearchState, DetailTab, Dialog, GameLogState, I18n,
    InstallProgress, InstanceSettingsEdit, LoaderUiState, ModSearchState, ModSource,
    ModpackSourceState, NewInstanceInput, NewInstanceMode, PendingInstall, SettingsTab,
    VersionsUiState,
};
use crate::theme;
use crate::toast::ToastQueue;

#[cfg(not(target_os = "macos"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowButton {
    Close,
    Maximize,
    Restore,
    Minimize,
}

pub use miao_core::auth::MS_CLIENT_ID;

const CF_API_KEY_DEFAULT: &str = "$2a$10$IlC78/YEUsegXBjlRMkHpOE/./ZkDhzPu0XmjgHkpCV4R3Kqzpc0m";

pub const CF_API_KEY_BUILTIN: &str = match option_env!("CURSEFORGE_API_KEY") {
    Some(s) if !s.is_empty() => s,
    _ => CF_API_KEY_DEFAULT,
};

pub struct MiaoApp {
    pub config: LauncherConfig,
    pub instances: Vec<Instance>,

    pub nav_stack: NavigationStack,
    pub active_dialog: Dialog,
    pub active_tab: DetailTab,
    pub selected_instance: Option<usize>,
    pub status: String,

    pub offline_username_input: String,
    pub offline_skin_model: miao_core::auth::SkinModel,
    pub cf_api_key_input: String,
    pub data_dir_input: String,
    pub new_instance: NewInstanceInput,
    pub mod_source: ModSource,
    pub mod_search_active: bool,
    pub mod_search_query: String,
    pub mod_search: ModSearchState,
    pub cf_search: CfSearchState,
    pub pending_install: Option<PendingInstall>,

    pub confirm_delete: Option<usize>,
    pub settings_tab: SettingsTab,
    pub cached_javas: Option<Vec<miao_core::java::JavaInstallation>>,
    /// True while a background Java detection task is in flight.
    pub java_detecting: bool,
    /// If set, automatically resume launching this instance once Java detection finishes.
    pub pending_launch_idx: Option<usize>,
    pub refresh_counter: u32,
    pub theme_preset: crate::theme::ThemePreset,
    pub custom_themes: Vec<miao_core::custom_theme::LoadedTheme>,
    pub active_custom_theme: Option<String>,

    pub instance_settings_edit: InstanceSettingsEdit,
    pub language: String,
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
    pub mod_updates: Vec<miao_core::modrinth::api::ModUpdateInfo>,
    pub checking_updates: bool,
    /// Modpack source UI state. Phase 1 PR 3.3 lays the field; PR 3.4 wires
    /// it to the new-instance modpack browser dialog.
    #[allow(dead_code)]
    pub modpack_source: ModpackSourceState,
    pub setup_step: SetupStep,

    pub blur_renderer: Arc<Mutex<BlurRenderer>>,
    pub toasts: ToastQueue,

    pub sidebar_indicator_y: Spring,
    pub tab_indicator_x: Spring,
    pub tab_indicator_width: Spring,
    pub settings_nav_indicator_y: Spring,
    pub smoothed_progress: f32,

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
        config.ensure_data_dirs();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        let gl = cc.gl.as_ref().expect("glow context required");
        let blur_renderer = Arc::new(Mutex::new(BlurRenderer::new(gl)));

        let rt = Arc::new(Runtime::new().expect("failed to create tokio runtime"));
        let controller = AppController::new(&rt, cc.egui_ctx.clone());

        controller.send(AppCommand::FetchVersionManifest {
            mirror: config.download_mirror.clone(),
        });
        controller.send(AppCommand::CheckForUpdates);
        // Warm the Java detection cache up-front so the first launch click is instant.
        controller.send(AppCommand::RefreshJava {
            data_dir: config.data_dir.clone(),
            force: false,
        });
        // Re-fetch avatars/capes for any non-offline accounts. This rebuilds the
        // skin cache after a data-dir wipe and keeps avatars current if the player
        // changed their skin between launches.
        for account in &config.accounts {
            if account.is_offline() {
                continue;
            }
            controller.send(AppCommand::RefreshSkin {
                uuid: account.uuid().as_simple().to_string(),
                data_dir: config.data_dir.clone(),
            });
        }

        let data_dir_input = config.data_dir.display().to_string();
        let mirror_custom_url = match &config.download_mirror {
            miao_core::config::DownloadMirror::Custom(url) => url.clone(),
            _ => String::new(),
        };
        let max_downloads_input = config.max_concurrent_downloads.to_string();
        let theme_preset = config.theme;
        let language = config.language.clone();
        I18n::load_external_locales(&config.locales_dir());
        let custom_themes = miao_core::custom_theme::load_themes_from_dir(&config.themes_dir());
        let active_custom_theme = config.custom_theme_name.clone();
        let cf_api_key_input = config.curseforge_api_key.clone().unwrap_or_default();

        Self {
            config,
            instances,
            nav_stack: NavigationStack::new(Page::Main),
            active_dialog: Dialog::None,
            active_tab: DetailTab::Mods,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            offline_skin_model: miao_core::auth::SkinModel::Classic,
            cf_api_key_input,
            data_dir_input,
            new_instance: NewInstanceInput::default(),
            mod_source: ModSource::default(),
            mod_search_active: false,
            mod_search_query: String::new(),
            mod_search: ModSearchState::default(),
            cf_search: CfSearchState::default(),
            pending_install: None,
            confirm_delete: None,
            settings_tab: SettingsTab::default(),
            cached_javas: None,
            java_detecting: false,
            pending_launch_idx: None,
            refresh_counter: 0,
            theme_preset,
            custom_themes,
            active_custom_theme,
            instance_settings_edit: InstanceSettingsEdit::default(),
            language,
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
            mod_updates: Vec::new(),
            checking_updates: false,
            modpack_source: ModpackSourceState::default(),
            setup_step: SetupStep::default(),

            blur_renderer,
            toasts: ToastQueue::new(),
            sidebar_indicator_y: Spring::new(0.0, PRESET_BOUNCY.spring),
            tab_indicator_x: Spring::new(0.0, PRESET_BOUNCY.spring),
            tab_indicator_width: Spring::new(60.0, PRESET_BOUNCY.spring),
            settings_nav_indicator_y: Spring::new(0.0, PRESET_BOUNCY.spring),
            smoothed_progress: 0.0,
            controller,
            rt,
        }
    }

    pub fn open_new_instance_dialog(&mut self) {
        self.active_dialog = Dialog::NewInstance;
        self.new_instance.reset();
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
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
                AppEvent::LoaderVersionsFetched { versions, failed } => {
                    self.loader.versions = versions;
                    self.loader.failed = failed;
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
                    self.install_progress.insert(
                        task_id,
                        InstallProgress {
                            completed,
                            total,
                            label,
                        },
                    );
                }
                AppEvent::InstallFinished {
                    task_id,
                    success,
                    message,
                } => {
                    self.active_installs.remove(&task_id);
                    self.install_progress.remove(&task_id);
                    self.smoothed_progress = 0.0;
                    self.status = message.clone();
                    if success {
                        self.toasts.success(&message);
                        self.instances = instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                        self.cached_javas = None;
                        miao_core::java::invalidate_java_cache();
                        // The instance install may have downloaded a new Java; refresh in
                        // the background so the next launch is instant.
                        self.controller.send(AppCommand::RefreshJava {
                            data_dir: self.config.data_dir.clone(),
                            force: true,
                        });
                        self.java_detecting = true;
                        self.selected_instance = Some(self.instances.len().saturating_sub(1));
                        self.active_tab = DetailTab::Mods;
                        if matches!(self.nav_stack.current(), Page::Settings) {
                            self.nav_stack.pop();
                        }
                    } else {
                        self.toasts.error(&message);
                    }
                }
                AppEvent::ModpackManifestFetched {
                    source_id,
                    manifest,
                } => {
                    self.modpack_source.manifest_loading = false;
                    self.modpack_source.manifest_error = None;
                    self.modpack_source.current_source_id = Some(source_id);
                    self.modpack_source.manifest = Some(manifest);
                    self.modpack_source.selected_pack_idx = None;
                    self.modpack_source.selected_mc_version = None;
                }
                AppEvent::ModpackManifestFailed { source_id, error } => {
                    self.modpack_source.manifest_loading = false;
                    self.modpack_source.current_source_id = Some(source_id);
                    self.modpack_source.manifest_error = Some(error.clone());
                    self.toasts.error(format!("Modpack manifest: {error}"));
                }
                AppEvent::ModpackInstallFinished {
                    success, message, ..
                } => {
                    self.status = message.clone();
                    if success {
                        self.toasts.success(&message);
                        self.instances = instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                        self.selected_instance = Some(self.instances.len().saturating_sub(1));
                        self.active_tab = DetailTab::Mods;
                    } else {
                        self.toasts.error(format!("Modpack install: {message}"));
                    }
                }
                AppEvent::DeviceCode(dc) => {
                    self.auth.device_code = Some(dc);
                }
                AppEvent::LoginComplete { account } => {
                    let msg = match &account {
                        AuthMethod::Microsoft(ms) => format!("Logged in as {}", ms.username),
                        AuthMethod::AuthlibInjector(a) => {
                            format!("Logged in as {} ({})", a.username, a.server_name)
                        }
                        AuthMethod::Offline(o) => format!("Added {}", o.username),
                    };
                    self.status = format!("✓ {}", msg);
                    self.toasts.success(&msg);
                    self.config.accounts.push(account);
                    if self.config.active_account_index.is_none() {
                        self.config.active_account_index = Some(0);
                    }
                    if let Err(e) = self.config.save() {
                        self.status = format!("✗ Failed to save account: {}", e);
                        self.toasts.error(format!("Failed to save account: {}", e));
                    }
                    self.auth.device_code = None;
                    self.auth.logging_in = false;
                    self.auth.authlib_logging_in = false;
                }
                AppEvent::LoginFailed(msg) => {
                    self.status = msg.clone();
                    self.toasts.error(&msg);
                    self.auth.device_code = None;
                    self.auth.logging_in = false;
                    self.auth.authlib_logging_in = false;
                }
                AppEvent::SkinRefreshed { uuid } => {
                    // The on-disk PNG was just rewritten. Drop egui's in-memory image
                    // cache for both file:// URIs so the next render pulls the fresh
                    // bytes. Without this, a previously rendered placeholder (or a
                    // stale skin) sticks until the app restarts.
                    let cache = miao_core::skin::SkinCache::new(&self.config.data_dir);
                    let avatar_uri = format!("file://{}", cache.avatar_path(&uuid).display());
                    let cape_uri = format!("file://{}", cache.cape_path(&uuid).display());
                    ctx.forget_image(&avatar_uri);
                    ctx.forget_image(&cape_uri);
                }
                AppEvent::JavaInstalled { launch_idx } => {
                    // A new Java was installed; cache is stale. Re-detect in the
                    // background, then resume the launch from the JavaDetected handler.
                    self.cached_javas = None;
                    miao_core::java::invalidate_java_cache();
                    self.pending_launch_idx = Some(launch_idx);
                    self.java_detecting = true;
                    self.controller.send(AppCommand::RefreshJava {
                        data_dir: self.config.data_dir.clone(),
                        force: true,
                    });
                    self.status = "✓ Java installed — launching game...".to_string();
                }
                AppEvent::JavaFailed(msg) => {
                    self.status = format!("✗ {}", msg);
                    self.toasts.error(&msg);
                }
                AppEvent::JavaDetected { installations } => {
                    self.cached_javas = Some(installations);
                    self.java_detecting = false;
                    if let Some(idx) = self.pending_launch_idx.take() {
                        self.launch_instance(idx);
                    }
                }
                AppEvent::ModSearchResults { hits, total_hits } => {
                    if self.mod_search.offset == 0 {
                        self.mod_search.results = hits;
                    } else {
                        self.mod_search.results.extend(hits);
                    }
                    self.mod_search.total_hits = total_hits;
                    self.mod_search.offset = self.mod_search.results.len() as u32;
                    self.mod_search.searching = false;
                }
                AppEvent::ModVersions(versions) => {
                    self.mod_search.versions = versions;
                    self.mod_search.searching = false;
                }
                AppEvent::ModPendingInstall(pending) => {
                    self.pending_install = Some(PendingInstall::Modrinth(pending));
                    self.mod_search.searching = false;
                }
                AppEvent::ModInstalled { count } => {
                    let msg = format!("Installed {} mod(s)", count);
                    self.status = msg.clone();
                    self.toasts.success(msg);
                    self.mod_search.searching = false;
                    self.file_scan_cache.invalidate();
                }
                AppEvent::ModError(msg) => {
                    self.status = msg.clone();
                    self.toasts.error(&msg);
                    self.mod_search.searching = false;
                }
                AppEvent::CfSearchResults { mods, total_count } => {
                    if self.cf_search.offset == 0 {
                        self.cf_search.results = mods;
                    } else {
                        self.cf_search.results.extend(mods);
                    }
                    self.cf_search.total_count = total_count;
                    self.cf_search.offset = self.cf_search.results.len() as u32;
                    self.cf_search.files.clear();
                    self.cf_search.selected_mod_id = None;
                    self.cf_search.selected_mod_name = None;
                    self.cf_search.searching = false;
                }
                AppEvent::CfFileVersions(files) => {
                    self.cf_search.files = files;
                    self.cf_search.searching = false;
                }
                AppEvent::CfPendingInstall {
                    mod_name,
                    deps,
                    instance_dir,
                    mc_version,
                    loader,
                    api_key,
                } => {
                    self.pending_install = Some(PendingInstall::CurseForge(CfPendingInstall {
                        mod_name,
                        deps,
                        instance_dir,
                        mc_version,
                        loader,
                        api_key,
                    }));
                    self.cf_search.searching = false;
                }
                AppEvent::CfInstalled { count } => {
                    let msg = format!("Installed {} mod(s) from CurseForge", count);
                    self.status = msg.clone();
                    self.toasts.success(msg);
                    self.cf_search.searching = false;
                    self.pending_install = None;
                    self.file_scan_cache.invalidate();
                }
                AppEvent::CfError(msg) => {
                    self.status = msg.clone();
                    self.toasts.error(&msg);
                    self.cf_search.searching = false;
                }
                AppEvent::ModUpdatesResult { updates } => {
                    self.checking_updates = false;
                    if updates.is_empty() {
                        self.toasts
                            .success(I18n::t(&self.language, "up_to_date").to_string());
                    } else {
                        let msg = format!(
                            "{} {}",
                            updates.len(),
                            I18n::t(&self.language, "updates_available")
                        );
                        self.toasts.success(msg);
                    }
                    self.mod_updates = updates;
                }
                AppEvent::GameLogLine(line) => {
                    if self.game_log.lines.len() >= GameLogState::MAX_LINES {
                        self.game_log.lines.pop_front();
                    }
                    self.game_log.lines.push_back(line);
                    self.game_log.running = true;
                }
                AppEvent::GameExited { exit_code } => {
                    self.game_log.running = false;
                    match exit_code {
                        Some(0) => {
                            self.status = "Game exited normally.".to_string();
                        }
                        Some(code) => {
                            let msg = format!("Game crashed (exit code {})", code);
                            self.status = msg.clone();
                            self.toasts.error(msg);
                        }
                        None => {
                            self.status = "Game process terminated.".to_string();
                            self.toasts.warning("Game process was terminated");
                        }
                    }
                }
                AppEvent::ExportResult(msg) => {
                    self.status = msg;
                }
                AppEvent::ImportResult { success, message } => {
                    self.status = message;
                    if success {
                        self.instances = instance::list_instances(&self.config.instances_dir())
                            .unwrap_or_default();
                        self.selected_instance = Some(self.instances.len().saturating_sub(1));
                        self.active_tab = DetailTab::Mods;
                        if matches!(self.nav_stack.current(), Page::Settings) {
                            self.nav_stack.pop();
                        }
                    }
                }
                AppEvent::UpdateAvailable(version) => {
                    self.update_available = Some(version);
                }
                AppEvent::TaskCancelled { task_id } => {
                    self.active_installs.remove(&task_id);
                    self.install_progress.remove(&task_id);
                    self.smoothed_progress = 0.0;
                    self.status = "Cancelled.".to_string();
                }
                AppEvent::Error(msg) => {
                    self.status = msg.clone();
                    self.toasts.error(&msg);
                }
            }
        }
    }
}

impl eframe::App for MiaoApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Fully transparent background so rounded window corners show through
        egui::Rgba::TRANSPARENT.to_array()
    }

    #[allow(deprecated)]
    fn ui(&mut self, _root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = _root_ui.ctx();
        self.drain_events(ctx);
        if let Some(ref name) = self.active_custom_theme {
            if let Some(loaded) = self.custom_themes.iter().find(|t| &t.file_name == name) {
                theme::apply_custom_theme(ctx, &loaded.theme.colors);
            } else {
                theme::apply_theme(ctx, self.theme_preset);
            }
        } else {
            theme::apply_theme(ctx, self.theme_preset);
        }

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
            || self.java_detecting
            || self.nav_stack.is_transitioning()
        {
            ctx.request_repaint();
        }

        let _transition_alpha = self.nav_stack.animate(ctx);

        let dt = ctx.input(|i| i.stable_dt);
        self.sidebar_indicator_y.tick(dt);
        self.tab_indicator_x.tick(dt);
        self.tab_indicator_width.tick(dt);
        self.settings_nav_indicator_y.tick(dt);
        if !self.sidebar_indicator_y.is_settled()
            || !self.tab_indicator_x.is_settled()
            || !self.tab_indicator_width.is_settled()
            || !self.settings_nav_indicator_y.is_settled()
        {
            ctx.request_repaint();
        }

        // Paint rounded window background; corners remain transparent
        let window_rect = ctx.input(|i| i.screen_rect());
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            window_rect,
            theme::Radii::WINDOW,
            theme::Colors::bg_main(),
        );

        egui::TopBottomPanel::bottom("status_bar")
            .frame(theme::bottom_bar_frame())
            .show(ctx, |ui| {
                let active_progress = self.install_progress.values().next();
                if let Some(progress) = active_progress {
                    if progress.total > 0 {
                        let target = progress.completed as f32 / progress.total.max(1) as f32;
                        let speed = 8.0;
                        self.smoothed_progress +=
                            (target - self.smoothed_progress) * (speed * dt).min(1.0);
                        if (self.smoothed_progress - target).abs() < 0.001 {
                            self.smoothed_progress = target;
                        }
                        let fraction = self.smoothed_progress;
                        let task_count = self.active_installs.len();
                        let text = if task_count > 1 {
                            format!(
                                "{} ({}/{}) [+{} tasks]",
                                progress.label,
                                progress.completed,
                                progress.total,
                                task_count - 1
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
                                    .fill(theme::Colors::accent()),
                            );
                            if ui.small_button(crate::icons::ICON_X).clicked() {
                                self.cancel_all_tasks();
                            }
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(theme::status_text(&self.status));
                            if ui.small_button(crate::icons::ICON_X).clicked() {
                                self.cancel_all_tasks();
                            }
                        });
                    }
                } else if !self.active_installs.is_empty() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(theme::status_text(&self.status));
                        if ui.small_button(crate::icons::ICON_X).clicked() {
                            self.cancel_all_tasks();
                        }
                    });
                } else {
                    ui.label(theme::status_text(&self.status));
                }
            });

        if !self.config.setup_complete {
            egui::TopBottomPanel::top("setup_top_bar")
                .frame(theme::top_bar_frame())
                .show(ctx, |ui| {
                    self.render_title_bar(ui, ctx, false);
                });
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::NONE
                        .fill(theme::Colors::bg_main())
                        .inner_margin(egui::Margin::same(12)),
                )
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(80.0);
                        self.render_setup_wizard(ui);
                    });
                });
            self.toasts.render(ctx);
            return;
        }

        egui::TopBottomPanel::top("top_bar")
            .frame(theme::top_bar_frame())
            .show(ctx, |ui| {
                self.render_title_bar(ui, ctx, false);
            });

        self.render_sidebar(ctx);

        let in_settings = matches!(self.nav_stack.current(), Page::Settings);
        let content_opacity = _transition_alpha;

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(theme::Colors::bg_main())
                    .inner_margin(if in_settings {
                        egui::Margin::same(0)
                    } else {
                        egui::Margin::same(12)
                    }),
            )
            .show(ctx, |ui| {
                ui.set_opacity(content_opacity);
                let slide_x = self.nav_stack.slide_offset();
                if slide_x.abs() > 0.1 {
                    let mut child_rect = ui.available_rect_before_wrap();
                    child_rect = child_rect.translate(egui::vec2(slide_x, 0.0));
                    ui.allocate_ui_at_rect(child_rect, |ui| {
                        if in_settings {
                            self.render_settings_page(ui);
                        } else {
                            self.render_detail(ui);
                        }
                    });
                } else if in_settings {
                    self.render_settings_page(ui);
                } else {
                    self.render_detail(ui);
                }
            });

        if self.active_dialog != Dialog::None {
            let screen = ctx.content_rect();
            egui::Area::new(egui::Id::new("dialog_blur_backdrop"))
                .fixed_pos(screen.min)
                .interactable(true)
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    ui.set_clip_rect(screen);
                    let tint = egui::Color32::from_black_alpha(120);
                    crate::blur::blur_behind(ui, &self.blur_renderer, screen, 12.0, tint);
                    if ui.allocate_rect(screen, egui::Sense::click()).clicked() {
                        self.active_dialog = Dialog::None;
                    }
                });
        }

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

        self.toasts.render(ctx);

        if self.toasts.has_active() {
            ctx.request_repaint();
        }
    }
}

impl MiaoApp {
    fn render_title_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, is_settings: bool) {
        let lang = self.language.clone();
        let height = 30.0;
        ui.set_min_height(height);
        let title_bar_rect = ui.max_rect();

        let title_bar_response = ui.interact(
            title_bar_rect,
            egui::Id::new("title_bar"),
            egui::Sense::click_and_drag(),
        );

        if title_bar_response.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if title_bar_response.double_clicked() {
            let is_max = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
        }

        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(title_bar_rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.add_space(platform::title_bar_left_padding());
                if is_settings {
                    if ui.button(I18n::t(&lang, "back")).clicked() {
                        self.nav_stack.pop();
                    }
                    ui.add_space(8.0);
                    ui.label(theme::heading(I18n::t(&lang, "settings")));
                } else {
                    ui.label(theme::heading("MMCL"));
                    ui.add_space(8.0);
                    ui.label(theme::small("MiaoMinecraftLauncher"));
                }
            },
        );

        self.render_custom_window_buttons(ui, ctx, title_bar_rect);
    }

    #[cfg(not(target_os = "macos"))]
    fn render_custom_window_buttons(
        &self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        title_bar_rect: egui::Rect,
    ) {
        debug_assert!(platform::should_render_custom_window_buttons());
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(title_bar_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;

                if Self::window_control_button(ui, WindowButton::Close).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }

                let is_maximized = ctx.input(|i| i.viewport().maximized).unwrap_or(false);
                if Self::window_control_button(
                    ui,
                    if is_maximized {
                        WindowButton::Restore
                    } else {
                        WindowButton::Maximize
                    },
                )
                .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
                }

                if Self::window_control_button(ui, WindowButton::Minimize).clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
            },
        );
    }

    #[cfg(target_os = "macos")]
    fn render_custom_window_buttons(
        &self,
        _ui: &mut egui::Ui,
        _ctx: &egui::Context,
        _title_bar_rect: egui::Rect,
    ) {
        debug_assert!(!platform::should_render_custom_window_buttons());
    }

    #[cfg(not(target_os = "macos"))]
    fn window_control_button(ui: &mut egui::Ui, button: WindowButton) -> egui::Response {
        let size = egui::vec2(46.0, 28.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        let hover_color = match button {
            WindowButton::Close => egui::Color32::from_rgb(196, 43, 28),
            _ => theme::Colors::bg_widget_hover(),
        };

        if response.hovered() {
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::ZERO, hover_color);
        }

        let icon_color = if response.hovered() && button == WindowButton::Close {
            egui::Color32::WHITE
        } else {
            theme::Colors::text_secondary()
        };

        let center = rect.center();
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.2, icon_color);

        match button {
            WindowButton::Close => {
                let d = 5.0;
                painter.line_segment(
                    [center - egui::vec2(d, d), center + egui::vec2(d, d)],
                    stroke,
                );
                painter.line_segment(
                    [center + egui::vec2(-d, d), center + egui::vec2(d, -d)],
                    stroke,
                );
            }
            WindowButton::Maximize => {
                let d = 5.0;
                painter.rect_stroke(
                    egui::Rect::from_center_size(center, egui::vec2(d * 2.0, d * 2.0)),
                    egui::CornerRadius::ZERO,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            WindowButton::Restore => {
                let d = 4.5;
                let offset = 2.0;
                let back = egui::Rect::from_min_size(
                    center + egui::vec2(-d + offset, -d - offset),
                    egui::vec2(d * 2.0 - offset, d * 2.0 - offset),
                );
                painter.rect_stroke(
                    back,
                    egui::CornerRadius::ZERO,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                let front = egui::Rect::from_min_size(
                    center + egui::vec2(-d, -d + offset),
                    egui::vec2(d * 2.0 - offset, d * 2.0 - offset),
                );
                painter.rect_filled(front, egui::CornerRadius::ZERO, theme::Colors::bg_dark());
                painter.rect_stroke(
                    front,
                    egui::CornerRadius::ZERO,
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            WindowButton::Minimize => {
                let d = 5.0;
                painter.line_segment(
                    [center + egui::vec2(-d, 0.0), center + egui::vec2(d, 0.0)],
                    stroke,
                );
            }
        }

        response
    }
}

impl MiaoApp {
    /// Request a fresh Java detection scan. Returns immediately; the result arrives via
    /// [`AppEvent::JavaDetected`]. Set `force` to bypass the cache.
    pub fn request_java_detection(&mut self, force: bool) {
        if self.java_detecting && !force {
            return;
        }
        self.java_detecting = true;
        self.controller.send(AppCommand::RefreshJava {
            data_dir: self.config.data_dir.clone(),
            force,
        });
    }

    pub fn launch_instance(&mut self, idx: usize) {
        if self.config.accounts.is_empty() || self.config.active_account_index.is_none() {
            let lang = self.language.clone();
            self.toasts.warning(I18n::t(&lang, "no_account_to_launch"));
            return;
        }

        // Ensure we have Java info before deciding what to do. If the cache is empty,
        // kick off a background scan and resume once it returns. UI stays responsive.
        let java_installations = match self.cached_javas.clone() {
            Some(list) => list,
            None => {
                self.pending_launch_idx = Some(idx);
                self.status = "Detecting Java installations...".to_string();
                self.request_java_detection(false);
                return;
            }
        };

        let inst = &self.instances[idx];

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
        // Use the cached list when available; if not, the upstream `launch_instance`
        // flow will have already kicked off a detection. Treat "no cache yet" as
        // "Java not yet known" and proceed to download regardless — the deduplication
        // below prevents accidental double-downloads.
        let already_have_java = self
            .cached_javas
            .as_ref()
            .and_then(|list| java::find_compatible_java(list, required))
            .is_some();
        if already_have_java {
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
            launch_idx: idx,
            config: self.config.clone(),
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

    pub fn get_available_loaders(&self) -> Vec<(usize, &'static str, bool, bool)> {
        let mut loaders = vec![(0, "None (Vanilla)", true, false)];
        for (idx, name, loader_type) in [
            (1, "Fabric", ModLoaderType::Fabric),
            (2, "Quilt", ModLoaderType::Quilt),
            (3, "NeoForge", ModLoaderType::NeoForge),
            (4, "Forge", ModLoaderType::Forge),
        ] {
            let available = self.loader.versions.contains_key(&loader_type);
            let fetch_failed = self.loader.failed.contains(&loader_type);
            loaders.push((idx, name, available, fetch_failed));
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
        self.status = I18n::t(&self.language, "saved").to_string();
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
