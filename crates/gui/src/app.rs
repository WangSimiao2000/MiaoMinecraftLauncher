use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{MicrosoftAuth, PollResult};
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::version::VersionInfo;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub use crate::state::{
    AppView, AsyncState, DetailTab, Dialog, I18n, InstanceSettingsEdit, Language, ModSearchState,
    NewInstanceInput, SettingsTab, SharedAsyncState, VersionsState,
};
use crate::theme;

pub const MS_CLIENT_ID: &str = "d3bbcbda-1e98-4ccd-9fc7-b107f30a5af8";

pub struct MiaoApp {
    pub config: LauncherConfig,
    pub instances: Vec<Instance>,
    pub async_state: SharedAsyncState,
    pub app_view: AppView,
    pub active_dialog: Dialog,
    pub active_tab: DetailTab,
    pub selected_instance: Option<usize>,
    pub status: String,

    pub offline_username_input: String,
    pub data_dir_input: String,
    pub new_instance: NewInstanceInput,
    pub mod_search: ModSearchState,

    pub confirm_delete: Option<usize>,
    pub settings_tab: SettingsTab,
    pub ctx: egui::Context,
    pub cached_javas: Option<Vec<miao_core::java::JavaInstallation>>,
    pub refresh_counter: u32,
    #[allow(dead_code)]
    pub theme_preset: crate::theme::ThemePreset,

    pub instance_settings_edit: InstanceSettingsEdit,
    pub language: Language,
    pub mirror_custom_url: String,
    pub max_downloads_input: String,
    pub rt: Arc<Runtime>,
}

impl MiaoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        let async_state = Arc::new(Mutex::new(AsyncState {
            versions: VersionsState {
                loading: true,
                ..Default::default()
            },
            ..Default::default()
        }));

        let rt = Arc::new(Runtime::new().expect("failed to create tokio runtime"));

        let state_clone = async_state.clone();
        let mirror = config.download_mirror.clone();
        let ctx = cc.egui_ctx.clone();

        rt.spawn(async move {
            let http = reqwest::Client::new();
            if let Ok(all_versions) =
                miao_core::version::manifest::fetch_version_manifest(&http, &mirror).await
            {
                let mut state = state_clone.lock().unwrap();
                let releases: Vec<_> = all_versions
                    .iter()
                    .filter(|v| v.is_release())
                    .cloned()
                    .collect();
                state.versions.all_versions = all_versions;
                state.versions.versions = releases;
                state.versions.loading = false;
            } else {
                state_clone.lock().unwrap().versions.loading = false;
            }
            ctx.request_repaint();
        });

        let data_dir_input = config.data_dir.display().to_string();
        let mirror_custom_url = match &config.download_mirror {
            miao_core::config::DownloadMirror::Custom(url) => url.clone(),
            _ => String::new(),
        };
        let max_downloads_input = config.max_concurrent_downloads.to_string();

        let app = Self {
            config,
            instances,
            async_state,
            app_view: AppView::Main,
            active_dialog: Dialog::None,
            active_tab: DetailTab::Mods,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            data_dir_input,
            new_instance: NewInstanceInput::default(),
            mod_search: ModSearchState::default(),
            confirm_delete: None,
            settings_tab: SettingsTab::default(),
            ctx: cc.egui_ctx.clone(),
            cached_javas: None,
            refresh_counter: 0,
            theme_preset: crate::theme::ThemePreset::Dark,
            instance_settings_edit: InstanceSettingsEdit::default(),
            language: Language::default(),
            mirror_custom_url,
            max_downloads_input,
            rt,
        };

        app.check_for_updates();
        app
    }

    pub fn open_new_instance_dialog(&mut self) {
        self.active_dialog = Dialog::NewInstance;
        self.new_instance.reset();
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

        if let Some(ref s) = state.install.status {
            self.status = s.clone();
            if s.starts_with('✓')
                || s.starts_with('✗')
                || (!state.install.installing && !state.auth.logging_in)
            {
                self.async_state.lock().unwrap().install.status = None;
                if s.starts_with('✓') {
                    self.instances =
                        instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
                    self.cached_javas = None;
                }
            }
        }

        if let Some(hits) = state.mod_search_hits.clone() {
            self.mod_search.results = hits;
            self.mod_search.searching = false;
            self.async_state.lock().unwrap().mod_search_hits = None;
        }
        if let Some(versions) = state.mod_versions.clone() {
            self.mod_search.versions = versions;
            self.mod_search.searching = false;
            self.async_state.lock().unwrap().mod_versions = None;
        }
        if state.pending_mod_install.is_some() {
            self.mod_search.searching = false;
        }

        if state.install.installing
            || state.auth.logging_in
            || state.loader.loading
            || self.mod_search.searching
            || state.game_log.running
        {
            ctx.request_repaint();
        }

        let lang = self.language;

        if let Some(launch_idx) = state.java_installed_launch_idx {
            self.async_state.lock().unwrap().java_installed_launch_idx = None;
            self.cached_javas = None;
            self.launch_instance(launch_idx);
        }

        egui::TopBottomPanel::bottom("status_bar")
            .frame(theme::bottom_bar_frame())
            .show(ctx, |ui| {
                if state.install.installing && state.install.progress_total > 0 {
                    let fraction = state.install.progress_completed as f32
                        / state.install.progress_total.max(1) as f32;
                    let text = format!(
                        "{} ({}/{})",
                        state.install.progress_label,
                        state.install.progress_completed,
                        state.install.progress_total
                    );
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .text(text)
                            .fill(theme::Colors::ACCENT),
                    );
                } else if state.install.installing {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(theme::status_text(&self.status));
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
                    self.render_settings_page(ui, &state);
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
                    Dialog::NewInstance => self.render_new_instance_dialog(ctx, &state),
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
        let account = self
            .config
            .accounts
            .first()
            .cloned()
            .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

        let java_installations = java::detect_java_with_data_dir(&self.config.data_dir);
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
            self.status = format!("Java {} not found. Confirm download?", required_java);
            self.active_dialog = Dialog::ConfirmJavaDownload {
                instance_idx: idx,
                java_major: required_java,
            };
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
                cmd.stdout(std::process::Stdio::piped());
                cmd.stderr(std::process::Stdio::piped());
                match cmd.spawn() {
                    Ok(child) => {
                        self.status = format!("Launched {}", inst.name);
                        {
                            let mut s = self.async_state.lock().unwrap();
                            s.game_log.lines.clear();
                            s.game_log.running = true;
                        }
                        let state = self.async_state.clone();
                        let ctx = self.ctx.clone();
                        std::thread::spawn(move || {
                            Self::stream_game_output(child, state, ctx);
                        });
                    }
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
        if java::find_compatible_java(
            &java::detect_java_with_data_dir(&self.config.data_dir),
            required,
        )
        .is_some()
        {
            self.status = format!("Java {} already available.", required);
            return;
        }

        {
            let mut s = self.async_state.lock().unwrap();
            if s.install.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            s.install.installing = true;
            s.install.status = Some(format!("Downloading Java {}...", required));
        }

        self.status = format!("Downloading Java {}...", required);
        let state = self.async_state.clone();
        let java_dir = self.config.data_dir.join("java");
        let launch_idx = idx;
        let ctx = self.ctx.clone();

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            let result = match miao_core::java::download::fetch_latest_asset(&http, required).await
            {
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
                            sp.lock().unwrap().install.status = Some(msg);
                        },
                    )
                    .await
                }
                Err(e) => Err(e),
            };

            let mut s = state.lock().unwrap();
            s.install.installing = false;
            match result {
                Ok(path) => {
                    s.install.status =
                        Some(format!("✓ Java {} installed — launching game...", required,));
                    s.java_installed_launch_idx = Some(launch_idx);
                    let _ = &path;
                }
                Err(e) => {
                    s.install.status = Some(format!("✗ Java download failed: {}", e));
                }
            }
            ctx.request_repaint();
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
            if s.install.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            s.install.installing = true;
            s.install.status = Some(format!("Creating '{}'...", name));
        }

        self.status = format!("Creating '{}'...", name);
        let state = self.async_state.clone();
        let config = self.config.clone();
        let ctx = self.ctx.clone();

        self.rt.spawn(async move {
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
            s.install.installing = false;
            s.install.progress_total = 0;
            s.install.progress_completed = 0;
            match result {
                Ok(_) => s.install.status = Some(format!("✓ '{}' created!", name)),
                Err(e) => s.install.status = Some(format!("✗ Failed: {}", e)),
            }
            ctx.request_repaint();
        });
    }

    pub fn start_ms_login(&mut self) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.auth.logging_in {
                return;
            }
            s.auth.logging_in = true;
        }

        let state = self.async_state.clone();
        let mut config = self.config.clone();
        let ctx = self.ctx.clone();

        self.rt.spawn(async move {
            let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());
            let device_code = match auth.request_device_code().await {
                Ok(dc) => dc,
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.auth.logging_in = false;
                    s.install.status = Some(format!("Login error: {}", e));
                    ctx.request_repaint();
                    return;
                }
            };

            let _ = open::that(&device_code.verification_uri);
            let code = device_code.device_code.clone();
            let interval = device_code.interval;
            state.lock().unwrap().auth.device_code = Some(device_code);
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
                                s.auth.device_code = None;
                                s.auth.logging_in = false;
                                s.install.status = Some(format!("✓ Logged in as {}", name));
                            }
                            Err(e) => {
                                let mut s = state.lock().unwrap();
                                s.auth.device_code = None;
                                s.auth.logging_in = false;
                                s.install.status = Some(format!("Auth error: {}", e));
                            }
                        }
                        ctx.request_repaint();
                        return;
                    }
                    Ok(PollResult::Pending) | Ok(PollResult::SlowDown) => continue,
                    Ok(PollResult::Expired) => {
                        let mut s = state.lock().unwrap();
                        s.auth.device_code = None;
                        s.auth.logging_in = false;
                        s.install.status = Some("Code expired.".to_string());
                        ctx.request_repaint();
                        return;
                    }
                    Ok(PollResult::Error(e)) => {
                        let mut s = state.lock().unwrap();
                        s.auth.device_code = None;
                        s.auth.logging_in = false;
                        s.install.status = Some(format!("Error: {}", e));
                        ctx.request_repaint();
                        return;
                    }
                    Err(_) => continue,
                }
            }
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
                s.install.status = Some("Export cancelled.".to_string());
                ctx.request_repaint();
                return;
            };

            {
                let mut s = state.lock().unwrap();
                s.install.status = Some(format!("Exporting '{}'...", inst.name));
            }
            ctx.request_repaint();

            match miao_core::modrinth::mrpack::export_mrpack(&instance_dir, &inst, &output_path) {
                Ok(path) => {
                    let mut s = state.lock().unwrap();
                    s.install.status = Some(format!("✓ Exported to {}", path.display()));
                }
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.install.status = Some(format!("✗ Export failed: {}", e));
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
                s.install.status = Some("Import cancelled.".to_string());
                ctx.request_repaint();
                return;
            };

            {
                let mut s = state.lock().unwrap();
                s.install.installing = true;
                s.install.status = Some(format!("Importing {}...", mrpack_path.display()));
            }
            ctx.request_repaint();

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
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
                        s.install.installing = false;
                        s.install.status = Some(format!(
                            "✓ Imported '{}' (MC {}{})",
                            inst.name, inst.minecraft_version, loader_info
                        ));
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.install.installing = false;
                        s.install.status = Some(format!("✗ Import failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    pub fn fetch_loader_versions(&mut self, mc_version: &str) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.loader.loading {
                return;
            }
            s.loader.loading = true;
            s.loader.versions.clear();
        }

        let state = self.async_state.clone();
        let mc_version = mc_version.to_string();
        let ctx = self.ctx.clone();

        self.rt.spawn(async move {
            let http = reqwest::Client::new();
            let versions =
                miao_core::modloader::fetch_all_loader_versions(&http, &mc_version).await;
            let mut s = state.lock().unwrap();
            s.loader.loading = false;
            if let Ok(v) = versions {
                s.loader.versions = v;
            }
            drop(s);
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
            loaders.push((idx, name, state.loader.versions.contains_key(&loader_type)));
        }
        loaders
    }

    pub fn get_loader_versions<'a>(&self, state: &'a AsyncState) -> Vec<&'a ModLoaderVersion> {
        if self.new_instance.loader == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.new_instance.loader - 1)
            .and_then(|lt| state.loader.versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    fn stream_game_output(
        mut child: std::process::Child,
        state: SharedAsyncState,
        ctx: egui::Context,
    ) {
        use crate::state::GameLogState;
        use std::io::BufRead;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let state2 = state.clone();
        let ctx2 = ctx.clone();

        let stdout_handle = stdout.map(|out| {
            let s = state.clone();
            let c = ctx.clone();
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(out);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let mut st = s.lock().unwrap();
                    if st.game_log.lines.len() >= GameLogState::MAX_LINES {
                        st.game_log.lines.pop_front();
                    }
                    st.game_log.lines.push_back(line);
                    drop(st);
                    c.request_repaint();
                }
            })
        });

        let stderr_handle = stderr.map(|err| {
            let s = state2.clone();
            let c = ctx2.clone();
            std::thread::spawn(move || {
                let reader = std::io::BufReader::new(err);
                for line in reader.lines() {
                    let Ok(line) = line else { break };
                    let mut st = s.lock().unwrap();
                    if st.game_log.lines.len() >= GameLogState::MAX_LINES {
                        st.game_log.lines.pop_front();
                    }
                    st.game_log.lines.push_back(format!("[ERR] {}", line));
                    drop(st);
                    c.request_repaint();
                }
            })
        });

        if let Some(h) = stdout_handle {
            let _ = h.join();
        }
        if let Some(h) = stderr_handle {
            let _ = h.join();
        }
        let _ = child.wait();
        state2.lock().unwrap().game_log.running = false;
        ctx2.request_repaint();
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

    pub fn check_for_updates(&self) {
        let state = self.async_state.clone();
        let ctx = self.ctx.clone();
        self.rt.spawn(async move {
            let Ok(http) = reqwest::Client::builder().user_agent("MMCL/0.1.0").build() else {
                return;
            };
            let url =
                "https://api.github.com/repos/WangSimiao2000/MiaoMinecraftLauncher/releases/latest";
            if let Ok(resp) = http.get(url).send().await
                && let Ok(json) = resp.json::<serde_json::Value>().await
                && let Some(tag) = json.get("tag_name").and_then(|v| v.as_str())
            {
                let current = env!("CARGO_PKG_VERSION");
                let remote = tag.trim_start_matches('v');
                if remote != current {
                    let mut s = state.lock().unwrap();
                    s.update_available = Some(tag.to_string());
                }
            }
            ctx.request_repaint();
        });
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
        s.install.progress_total = tasks.len();
        s.install.progress_completed = 0;
        s.install.progress_label = "Downloading libraries".to_string();
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
        s.install.progress_completed = p.completed_files;
        s.install.progress_total = p.total_files;
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
            s.install.progress_total = asset_tasks.len();
            s.install.progress_completed = 0;
            s.install.progress_label = "Downloading assets".to_string();
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
            s.install.progress_completed = p.completed_files;
            s.install.progress_total = p.total_files;
            ctx_cb2.request_repaint();
        }));
        dm2.download_all(asset_tasks).await?;
    }

    {
        let mut s = state.lock().unwrap();
        s.install.progress_total = 0;
        s.install.progress_completed = 0;
        s.install.progress_label.clear();
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
