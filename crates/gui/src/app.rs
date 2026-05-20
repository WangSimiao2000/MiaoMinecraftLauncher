use eframe::egui;
use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{DeviceCodeResponse, MicrosoftAuth, PollResult};
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::version::VersionInfo;
use std::sync::{Arc, Mutex};

const MS_CLIENT_ID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Instances,
    Versions,
    Accounts,
    Settings,
}

#[derive(Debug, Clone, Default)]
struct AsyncState {
    versions: Vec<VersionInfo>,
    versions_loading: bool,
    install_status: Option<String>,
    installing: bool,
    ms_device_code: Option<DeviceCodeResponse>,
    ms_logging_in: bool,
}

pub struct MiaoApp {
    config: LauncherConfig,
    instances: Vec<Instance>,
    async_state: Arc<Mutex<AsyncState>>,
    active_panel: Panel,
    selected_instance: Option<usize>,
    selected_version: Option<usize>,
    status: String,
    offline_username_input: String,
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

        Self {
            config,
            instances,
            async_state,
            active_panel: Panel::Instances,
            selected_instance: None,
            selected_version: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
        }
    }

    fn launch_instance(&mut self, idx: usize) {
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
            self.status = format!("No Java {} found!", required_java);
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
            Ok(mut cmd) => match cmd.spawn() {
                Ok(_) => self.status = format!("Launched {}", inst.name),
                Err(e) => self.status = format!("Launch failed: {}", e),
            },
            Err(e) => self.status = format!("Command error: {}", e),
        }
    }

    fn install_version(&mut self, ver: VersionInfo) {
        {
            let mut state = self.async_state.lock().unwrap();
            if state.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            state.installing = true;
            state.install_status = Some(format!("Installing {}...", ver.id));
        }

        self.status = format!("Installing {}...", ver.id);

        let state = self.async_state.clone();
        let config = self.config.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = do_install(&ver, &config, &state).await;
                let mut s = state.lock().unwrap();
                s.installing = false;
                match result {
                    Ok(_) => s.install_status = Some(format!("✓ {} installed!", ver.id)),
                    Err(e) => s.install_status = Some(format!("✗ Failed: {}", e)),
                }
            });
        });
    }

    fn start_ms_login(&mut self, ctx: egui::Context) {
        {
            let mut state = self.async_state.lock().unwrap();
            if state.ms_logging_in {
                return;
            }
            state.ms_logging_in = true;
        }

        self.status = "Starting Microsoft login...".to_string();
        let state = self.async_state.clone();
        let mut config = self.config.clone();

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

                {
                    let mut s = state.lock().unwrap();
                    s.ms_device_code = Some(device_code);
                }
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
}

impl eframe::App for MiaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = self.async_state.lock().unwrap().clone();

        if let Some(ref s) = state.install_status {
            if s.starts_with('✓') && s.contains("installed") {
                self.instances =
                    instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
            }
            if !state.installing && !state.ms_logging_in {
                self.status = s.clone();
            }
        }

        if state.installing {
            if let Some(ref s) = state.install_status {
                self.status = s.clone();
            }
            ctx.request_repaint();
        }

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
            Panel::Versions => self.render_versions(ui, &state),
            Panel::Accounts => {
                let ctx_clone = ctx.clone();
                self.render_accounts(ui, &state, ctx_clone);
            }
            Panel::Settings => self.render_settings(ui),
        });
    }
}

impl MiaoApp {
    fn render_instances(&mut self, ui: &mut egui::Ui) {
        ui.heading("Game Instances");
        ui.separator();

        if self.instances.is_empty() {
            ui.label("No instances. Go to Versions tab to install one.");
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
                }
            }

            ui.separator();
            if let Some(idx) = self.selected_instance
                && ui.button("▶ Launch").clicked()
            {
                self.launch_instance(idx);
            }
        }
    }

    fn render_versions(&mut self, ui: &mut egui::Ui, state: &AsyncState) {
        ui.heading("Available Versions");
        ui.separator();

        if state.versions_loading {
            ui.label("Loading versions...");
            return;
        }

        if state.versions.is_empty() {
            ui.label("No versions available.");
            return;
        }

        if state.installing {
            ui.label("Installing... please wait.");
            if let Some(ref s) = state.install_status {
                ui.label(s);
            }
            ui.separator();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, ver) in state.versions.iter().enumerate() {
                ui.horizontal(|ui| {
                    let selected = self.selected_version == Some(i);
                    if ui
                        .selectable_label(selected, format!("{:<16}", ver.id))
                        .clicked()
                    {
                        self.selected_version = Some(i);
                    }
                    ui.label(&ver.release_time);
                    if !state.installing && ui.small_button("Install").clicked() {
                        self.install_version(ver.clone());
                    }
                });
            }
        });
    }

    fn render_accounts(&mut self, ui: &mut egui::Ui, state: &AsyncState, ctx: egui::Context) {
        ui.heading("Accounts");
        ui.separator();

        if self.config.accounts.is_empty() {
            ui.label("No accounts configured.");
        } else {
            for (i, acc) in self.config.accounts.iter().enumerate() {
                let active = self.config.active_account_index == Some(i);
                let marker = if active { "★" } else { " " };
                let acc_type = if acc.is_microsoft() {
                    "Microsoft"
                } else {
                    "Offline"
                };
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
                self.status = format!("✓ Added: {}", account.username);
                self.config.accounts.push(AuthMethod::Offline(account));
                if self.config.active_account_index.is_none() {
                    self.config.active_account_index = Some(0);
                }
                let _ = self.config.save();
                self.offline_username_input.clear();
            }
        });

        ui.separator();
        ui.heading("Microsoft Login");

        if let Some(ref dc) = state.ms_device_code {
            ui.label(format!("Go to: {}", dc.verification_uri));
            ui.label(format!("Enter code: {}", dc.user_code));
            ui.label("Waiting for authorization...");
        } else if state.ms_logging_in {
            ui.label("Connecting...");
        } else if ui.button("Login with Microsoft").clicked() {
            self.start_ms_login(ctx);
        }
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.separator();
        ui.label(format!("Data: {}", self.config.data_dir.display()));
        ui.label(format!("Mirror: {:?}", self.config.download_mirror));
        ui.label(format!(
            "Concurrent downloads: {}",
            self.config.max_concurrent_downloads
        ));
        ui.separator();
        ui.label("Java:");
        let javas = java::detect_system_java();
        if javas.is_empty() {
            ui.label("  None detected");
        } else {
            for j in &javas {
                ui.label(format!(
                    "  Java {} ({}) - {}",
                    j.major_version,
                    j.version,
                    j.path.display()
                ));
            }
        }
    }
}

async fn do_install(
    ver: &VersionInfo,
    config: &LauncherConfig,
    state: &Arc<Mutex<AsyncState>>,
) -> anyhow::Result<()> {
    let http = reqwest::Client::new();

    {
        let mut s = state.lock().unwrap();
        s.install_status = Some(format!("Fetching metadata for {}...", ver.id));
    }

    let meta =
        miao_core::version::install::fetch_version_meta(&http, &ver.url, &config.download_mirror)
            .await?;

    miao_core::version::install::save_version_meta(&meta, config)?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, config, &config.download_mirror);
    let task_count = tasks.len();

    {
        let mut s = state.lock().unwrap();
        s.install_status = Some(format!("Downloading {} libraries...", task_count));
    }

    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
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
            s.install_status = Some(format!("Downloading {} assets...", asset_tasks.len()));
        }

        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks).await?;
    }

    let inst = Instance::new(&ver.id, &ver.id);
    let instance_dir = Instance::instance_dir(&config.instances_dir(), &ver.id);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    Ok(())
}
