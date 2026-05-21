use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{DeviceCodeResponse, MicrosoftAuth, PollResult};
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::version::VersionInfo;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::theme;

pub const MS_CLIENT_ID: &str = "d3bbcbda-1e98-4ccd-9fc7-b107f30a5af8";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Main,
    Settings,
    Welcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Mods,
    Resources,
    Worlds,
    Log,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    None,
    NewInstance,
}

#[derive(Debug, Clone, Default)]
pub struct AsyncState {
    pub versions: Vec<VersionInfo>,
    pub versions_loading: bool,
    pub install_status: Option<String>,
    pub installing: bool,
    pub ms_device_code: Option<DeviceCodeResponse>,
    pub ms_logging_in: bool,
    pub loader_versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    pub loading_loader_versions: bool,
    pub mod_search_hits: Option<Vec<miao_core::modrinth::api::SearchHit>>,
    pub mod_versions: Option<Vec<miao_core::modrinth::api::ProjectVersion>>,
    pub progress_total: usize,
    pub progress_completed: usize,
    pub progress_label: String,
}

pub struct MiaoApp {
    pub config: LauncherConfig,
    pub instances: Vec<Instance>,
    pub async_state: Arc<Mutex<AsyncState>>,
    pub app_view: AppView,
    pub active_dialog: Dialog,
    pub active_tab: DetailTab,
    pub selected_instance: Option<usize>,
    pub status: String,

    pub offline_username_input: String,
    pub data_dir_input: String,
    pub new_instance_name: String,
    pub new_instance_version_idx: usize,
    pub new_instance_loader: usize,
    pub new_instance_loader_version_idx: usize,

    pub mod_search_query: String,
    pub mod_search_results: Vec<miao_core::modrinth::api::SearchHit>,
    pub mod_search_versions: Vec<miao_core::modrinth::api::ProjectVersion>,
    pub mod_search_selected: usize,
    pub mod_searching: bool,
    pub mod_search_active: bool,

    pub ctx: egui::Context,
    pub cached_javas: Option<Vec<miao_core::java::JavaInstallation>>,
    pub refresh_counter: u32,
}

impl MiaoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        let async_state = Arc::new(Mutex::new(AsyncState {
            versions_loading: true,
            ..Default::default()
        }));

        let state_clone = async_state.clone();
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
                    let mut state = state_clone.lock().unwrap();
                    state.versions = releases;
                    state.versions_loading = false;
                } else {
                    state_clone.lock().unwrap().versions_loading = false;
                }
                ctx.request_repaint();
            });
        });

        let is_first_run = !config.data_dir.join("config.toml").exists() && instances.is_empty();
        let data_dir_input = config.data_dir.display().to_string();

        Self {
            config,
            instances,
            async_state,
            app_view: if is_first_run {
                AppView::Welcome
            } else {
                AppView::Main
            },
            active_dialog: Dialog::None,
            active_tab: DetailTab::Mods,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            data_dir_input,
            new_instance_name: String::new(),
            new_instance_version_idx: 0,
            new_instance_loader: 0,
            new_instance_loader_version_idx: 0,
            mod_search_query: String::new(),
            mod_search_results: Vec::new(),
            mod_search_versions: Vec::new(),
            mod_search_selected: 0,
            mod_searching: false,
            mod_search_active: false,
            ctx: cc.egui_ctx.clone(),
            cached_javas: None,
            refresh_counter: 0,
        }
    }

    pub fn open_new_instance_dialog(&mut self) {
        self.active_dialog = Dialog::NewInstance;
        self.new_instance_name.clear();
        self.new_instance_version_idx = 0;
        self.new_instance_loader = 0;
        self.new_instance_loader_version_idx = 0;
    }
}

impl eframe::App for MiaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        let state = self.async_state.lock().unwrap().clone();

        if let Some(ref s) = state.install_status {
            self.status = s.clone();
            if s.starts_with('✓')
                || s.starts_with('✗')
                || (!state.installing && !state.ms_logging_in)
            {
                self.async_state.lock().unwrap().install_status = None;
                if s.starts_with('✓') {
                    self.instances =
                        instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
                }
            }
        }

        if let Some(hits) = state.mod_search_hits.clone() {
            self.mod_search_results = hits;
            self.mod_searching = false;
            self.async_state.lock().unwrap().mod_search_hits = None;
        }
        if let Some(versions) = state.mod_versions.clone() {
            self.mod_search_versions = versions;
            self.mod_searching = false;
            self.async_state.lock().unwrap().mod_versions = None;
        }

        if state.installing
            || state.ms_logging_in
            || state.loading_loader_versions
            || self.mod_searching
        {
            ctx.request_repaint();
        }

        match self.app_view {
            AppView::Welcome => {
                self.render_welcome_setup(ctx);
            }
            AppView::Settings => {
                egui::TopBottomPanel::top("top_bar")
                    .frame(theme::top_bar_frame())
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("< Back").clicked() {
                                self.app_view = AppView::Main;
                            }
                            ui.add_space(8.0);
                            ui.label(theme::heading("Settings"));
                        });
                    });
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_settings_page(ui, &state);
                });
            }
            AppView::Main => {
                egui::TopBottomPanel::top("top_bar")
                    .frame(theme::top_bar_frame())
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(theme::heading("MiaoMC"));
                            ui.add_space(8.0);
                            ui.label(theme::small("Minecraft Launcher"));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.button("Settings").clicked() {
                                        self.app_view = AppView::Settings;
                                    }
                                },
                            );
                        });
                    });

                egui::TopBottomPanel::bottom("status_bar")
                    .frame(theme::bottom_bar_frame())
                    .show(ctx, |ui| {
                        if state.installing && state.progress_total > 0 {
                            let fraction = state.progress_completed as f32
                                / state.progress_total.max(1) as f32;
                            let text = format!(
                                "{} ({}/{})",
                                state.progress_label,
                                state.progress_completed,
                                state.progress_total
                            );
                            ui.add(
                                egui::ProgressBar::new(fraction)
                                    .text(text)
                                    .fill(theme::Colors::ACCENT),
                            );
                        } else {
                            ui.label(theme::status_text(&self.status));
                        }
                    });

                self.render_sidebar(ctx);

                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_detail(ui);
                });

                match self.active_dialog {
                    Dialog::NewInstance => self.render_new_instance_dialog(ctx, &state),
                    Dialog::None => {}
                }
            }
        }
    }
}

impl MiaoApp {
    pub fn launch_instance(&mut self, idx: usize) {
        let inst = &self.instances[idx];
        let account = self
            .config
            .accounts
            .first()
            .cloned()
            .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

        let java_installations = java::detect_system_java();
        let meta_path = self
            .config
            .versions_dir()
            .join(&inst.minecraft_version)
            .join(format!("{}.json", &inst.minecraft_version));

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

        let Some(java_path) = java_path else {
            self.status = format!("No Java {} found! Click 'Java' to download.", required_java);
            return;
        };

        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let options = LaunchOptions {
            game_dir: instance_dir,
            java_path: java_path.clone(),
            version_meta: meta,
            instance: inst.clone(),
            auth: account,
            config: self.config.clone(),
        };

        match build_launch_command(&options) {
            Ok(mut cmd) => {
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
                match cmd.spawn() {
                    Ok(_) => self.status = format!("Launched {}", inst.name),
                    Err(e) => self.status = format!("Launch failed: {}", e),
                }
            }
            Err(e) => self.status = format!("Command error: {}", e),
        }
    }

    pub fn download_java_for_instance(&mut self, idx: usize) {
        let inst = &self.instances[idx];
        let meta_path = self
            .config
            .versions_dir()
            .join(&inst.minecraft_version)
            .join(format!("{}.json", &inst.minecraft_version));

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
        if java::find_compatible_java(&java::detect_system_java(), required).is_some() {
            self.status = format!("Java {} already available.", required);
            return;
        }

        {
            let mut s = self.async_state.lock().unwrap();
            if s.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            s.installing = true;
            s.install_status = Some(format!("Downloading Java {}...", required));
        }

        self.status = format!("Downloading Java {}...", required);
        let state = self.async_state.clone();
        let java_dir = self.config.data_dir.join("java");

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let http = reqwest::Client::new();
                let result =
                    match miao_core::java::download::fetch_latest_asset(&http, required).await {
                        Ok(asset) => {
                            let sp = state.clone();
                            miao_core::java::download::download_and_extract_java_with_progress(
                                &http,
                                &asset,
                                &java_dir,
                                move |phase| {
                                    use miao_core::java::download::DownloadPhase;
                                    let msg = match phase {
                                        DownloadPhase::Downloading { downloaded, total } => {
                                            format!(
                                                "Java {}: {:.1}/{:.1} MB",
                                                required,
                                                downloaded as f64 / 1_000_000.0,
                                                total as f64 / 1_000_000.0
                                            )
                                        }
                                        DownloadPhase::Extracting => {
                                            format!("Java {}: extracting...", required)
                                        }
                                    };
                                    sp.lock().unwrap().install_status = Some(msg);
                                },
                            )
                            .await
                        }
                        Err(e) => Err(e),
                    };

                let mut s = state.lock().unwrap();
                s.installing = false;
                match result {
                    Ok(path) => {
                        s.install_status = Some(format!(
                            "✓ Java {} installed at {}",
                            required,
                            path.display()
                        ));
                    }
                    Err(e) => {
                        s.install_status = Some(format!("✗ Java download failed: {}", e));
                    }
                }
            });
        });
    }

    pub fn create_instance(
        &mut self,
        ver: VersionInfo,
        name: String,
        loader: Option<(String, String)>,
    ) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            s.installing = true;
            s.install_status = Some(format!("Creating '{}'...", name));
        }

        self.status = format!("Creating '{}'...", name);
        let state = self.async_state.clone();
        let config = self.config.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = do_create_instance(
                    &ver,
                    &name,
                    loader.as_ref().map(|(lt, lv)| (lt.as_str(), lv.as_str())),
                    &config,
                    state.clone(),
                    ctx.clone(),
                )
                .await;
                let mut s = state.lock().unwrap();
                s.installing = false;
                s.progress_total = 0;
                s.progress_completed = 0;
                match result {
                    Ok(_) => s.install_status = Some(format!("✓ '{}' created!", name)),
                    Err(e) => s.install_status = Some(format!("✗ Failed: {}", e)),
                }
                ctx.request_repaint();
            });
        });
    }

    pub fn start_ms_login(&mut self) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.ms_logging_in {
                return;
            }
            s.ms_logging_in = true;
        }

        let state = self.async_state.clone();
        let mut config = self.config.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());
                let device_code = match auth.request_device_code().await {
                    Ok(dc) => dc,
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.ms_logging_in = false;
                        s.install_status = Some(format!("Login error: {}", e));
                        ctx.request_repaint();
                        return;
                    }
                };

                let _ = open::that(&device_code.verification_uri);
                let code = device_code.device_code.clone();
                let interval = device_code.interval;
                state.lock().unwrap().ms_device_code = Some(device_code);
                ctx.request_repaint();

                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
                    match auth.poll_for_token(&code).await {
                        Ok(PollResult::Success(access_token, refresh_token)) => {
                            match auth
                                .authenticate_with_microsoft_token(
                                    &access_token,
                                    refresh_token.as_deref(),
                                )
                                .await
                            {
                                Ok(account) => {
                                    let name = account.username.clone();
                                    config.accounts.push(AuthMethod::Microsoft(account));
                                    if config.active_account_index.is_none() {
                                        config.active_account_index = Some(0);
                                    }
                                    let _ = config.save();
                                    let mut s = state.lock().unwrap();
                                    s.ms_device_code = None;
                                    s.ms_logging_in = false;
                                    s.install_status = Some(format!("✓ Logged in as {}", name));
                                }
                                Err(e) => {
                                    let mut s = state.lock().unwrap();
                                    s.ms_device_code = None;
                                    s.ms_logging_in = false;
                                    s.install_status = Some(format!("Auth error: {}", e));
                                }
                            }
                            ctx.request_repaint();
                            return;
                        }
                        Ok(PollResult::Pending) | Ok(PollResult::SlowDown) => continue,
                        Ok(PollResult::Expired) => {
                            let mut s = state.lock().unwrap();
                            s.ms_device_code = None;
                            s.ms_logging_in = false;
                            s.install_status = Some("Code expired.".to_string());
                            ctx.request_repaint();
                            return;
                        }
                        Ok(PollResult::Error(e)) => {
                            let mut s = state.lock().unwrap();
                            s.ms_device_code = None;
                            s.ms_logging_in = false;
                            s.install_status = Some(format!("Error: {}", e));
                            ctx.request_repaint();
                            return;
                        }
                        Err(_) => continue,
                    }
                }
            });
        });
    }

    pub fn export_instance_with_dialog(&mut self, idx: usize) {
        let inst = self.instances[idx].clone();
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();

        self.status = "Selecting export folder...".to_string();
        std::thread::spawn(move || {
            let folder = rfd::FileDialog::new()
                .set_title("Export .mrpack")
                .pick_folder();

            let Some(output_path) = folder else {
                let mut s = state.lock().unwrap();
                s.install_status = Some("Export cancelled.".to_string());
                ctx.request_repaint();
                return;
            };

            {
                let mut s = state.lock().unwrap();
                s.install_status = Some(format!("Exporting '{}'...", inst.name));
            }
            ctx.request_repaint();

            match miao_core::modrinth::mrpack::export_mrpack(&instance_dir, &inst, &output_path) {
                Ok(path) => {
                    let mut s = state.lock().unwrap();
                    s.install_status = Some(format!("✓ Exported to {}", path.display()));
                }
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.install_status = Some(format!("✗ Export failed: {}", e));
                }
            }
            ctx.request_repaint();
        });
    }

    pub fn import_with_dialog(&mut self) {
        let state = self.async_state.clone();
        let config = self.config.clone();
        let ctx = self.ctx.clone();

        self.status = "Selecting .mrpack file...".to_string();
        std::thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .set_title("Import .mrpack")
                .add_filter("Modrinth Modpack", &["mrpack"])
                .pick_file();

            let Some(mrpack_path) = file else {
                let mut s = state.lock().unwrap();
                s.install_status = Some("Import cancelled.".to_string());
                ctx.request_repaint();
                return;
            };

            {
                let mut s = state.lock().unwrap();
                s.installing = true;
                s.install_status = Some(format!("Importing {}...", mrpack_path.display()));
            }
            ctx.request_repaint();

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                match miao_core::modrinth::mrpack::import_mrpack(&mrpack_path, &config, None).await
                {
                    Ok(inst) => {
                        let loader_info = inst
                            .mod_loader
                            .as_ref()
                            .map(|l| format!(" + {} {}", l.loader_type, l.version))
                            .unwrap_or_default();
                        let mut s = state.lock().unwrap();
                        s.installing = false;
                        s.install_status = Some(format!(
                            "✓ Imported '{}' (MC {}{})",
                            inst.name, inst.minecraft_version, loader_info
                        ));
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.installing = false;
                        s.install_status = Some(format!("✗ Import failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    pub fn fetch_loader_versions(&mut self, mc_version: &str) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.loading_loader_versions {
                return;
            }
            s.loading_loader_versions = true;
            s.loader_versions.clear();
        }

        let state = self.async_state.clone();
        let mc_version = mc_version.to_string();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let http = reqwest::Client::new();
            rt.block_on(async {
                let versions =
                    miao_core::modloader::fetch_all_loader_versions(&http, &mc_version).await;
                let mut s = state.lock().unwrap();
                s.loading_loader_versions = false;
                if let Ok(v) = versions {
                    s.loader_versions = v;
                }
            });
            ctx.request_repaint();
        });
    }

    pub fn get_available_loaders(&self, state: &AsyncState) -> Vec<(usize, &'static str, bool)> {
        let mut loaders = vec![(0, "None (Vanilla)", true)];
        for (idx, name, loader_type) in [
            (1, "Fabric", ModLoaderType::Fabric),
            (2, "Quilt", ModLoaderType::Quilt),
            (3, "NeoForge", ModLoaderType::NeoForge),
            (4, "Forge", ModLoaderType::Forge),
        ] {
            loaders.push((idx, name, state.loader_versions.contains_key(&loader_type)));
        }
        loaders
    }

    pub fn get_loader_versions<'a>(&self, state: &'a AsyncState) -> Vec<&'a ModLoaderVersion> {
        if self.new_instance_loader == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.new_instance_loader - 1)
            .and_then(|lt| state.loader_versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }
}

async fn do_create_instance(
    ver: &VersionInfo,
    instance_name: &str,
    loader: Option<(&str, &str)>,
    config: &LauncherConfig,
    state: Arc<Mutex<AsyncState>>,
    ctx: egui::Context,
) -> anyhow::Result<()> {
    use miao_core::modloader::ModLoaderType;

    let http = reqwest::Client::new();

    let meta =
        miao_core::version::install::fetch_version_meta(&http, &ver.url, &config.download_mirror)
            .await?;
    miao_core::version::install::save_version_meta(&meta, config)?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, config, &config.download_mirror);

    {
        let mut s = state.lock().unwrap();
        s.progress_total = tasks.len();
        s.progress_completed = 0;
        s.progress_label = "Downloading libraries".to_string();
    }
    ctx.request_repaint();

    let state_cb = state.clone();
    let ctx_cb = ctx.clone();
    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    )
    .with_progress_callback(std::sync::Arc::new(move |p| {
        let mut s = state_cb.lock().unwrap();
        s.progress_completed = p.completed_files;
        s.progress_total = p.total_files;
        ctx_cb.request_repaint();
    }));
    dm.download_all(tasks).await?;

    let asset_index_path = config
        .assets_dir()
        .join("indexes")
        .join(format!("{}.json", &meta.asset_index.id));

    if asset_index_path.exists() {
        let asset_index = miao_core::version::assets::fetch_asset_index(&asset_index_path).await?;
        let asset_tasks = miao_core::version::assets::collect_asset_downloads(
            &asset_index,
            config,
            &config.download_mirror,
        );

        {
            let mut s = state.lock().unwrap();
            s.progress_total = asset_tasks.len();
            s.progress_completed = 0;
            s.progress_label = "Downloading assets".to_string();
        }
        ctx.request_repaint();

        let state_cb2 = state.clone();
        let ctx_cb2 = ctx.clone();
        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        )
        .with_progress_callback(std::sync::Arc::new(move |p| {
            let mut s = state_cb2.lock().unwrap();
            s.progress_completed = p.completed_files;
            s.progress_total = p.total_files;
            ctx_cb2.request_repaint();
        }));
        dm2.download_all(asset_tasks).await?;
    }

    {
        let mut s = state.lock().unwrap();
        s.progress_total = 0;
        s.progress_completed = 0;
        s.progress_label.clear();
    }

    let mut inst = Instance::new(instance_name, &ver.id);

    if let Some((loader_type_str, loader_version)) = loader {
        let lt = ModLoaderType::ALL
            .iter()
            .find(|t| t.as_str() == loader_type_str)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown loader: {}", loader_type_str))?;

        let loader_config =
            miao_core::modloader::install_loader(&http, &lt, &ver.id, loader_version, config)
                .await?;
        inst.mod_loader = Some(loader_config);
    }

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    Ok(())
}
