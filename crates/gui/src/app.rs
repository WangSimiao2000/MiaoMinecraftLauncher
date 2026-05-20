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
    loader_versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    loading_loader_versions: bool,
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
    selected_loader: usize,
    show_create_dialog: bool,
    new_instance_name: String,
    new_instance_version_idx: usize,
    new_instance_loader: usize,
    new_instance_loader_version_idx: usize,
}

const LOADER_NAMES: [&str; 4] = ["Fabric", "Quilt", "NeoForge", "Forge"];

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
            selected_loader: 0,
            show_create_dialog: false,
            new_instance_name: String::new(),
            new_instance_version_idx: 0,
            new_instance_loader: 0,
            new_instance_loader_version_idx: 0,
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
            self.status = format!(
                "No Java {} found! Click '☕ Download Java' to install.",
                required_java
            );
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

    fn install_loader(&mut self, instance_name: String, mc_version: String, loader_idx: usize) {
        {
            let mut state = self.async_state.lock().unwrap();
            if state.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            state.installing = true;
            state.install_status = Some(format!(
                "Installing {} for '{}'...",
                LOADER_NAMES[loader_idx], instance_name
            ));
        }

        self.status = format!("Installing {}...", LOADER_NAMES[loader_idx]);

        let state = self.async_state.clone();
        let config = self.config.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result =
                    do_loader_install(&instance_name, &mc_version, loader_idx, &config).await;
                let mut s = state.lock().unwrap();
                s.installing = false;
                match result {
                    Ok(msg) => s.install_status = Some(format!("✓ {}", msg)),
                    Err(e) => s.install_status = Some(format!("✗ Loader failed: {}", e)),
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
                self.async_state.lock().unwrap().install_status = None;
            } else if !state.installing && !state.ms_logging_in {
                self.status = s.clone();
                self.async_state.lock().unwrap().install_status = None;
            } else if state.installing {
                self.status = s.clone();
            }
        }

        if state.installing || state.ms_logging_in {
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

        if self.show_create_dialog {
            self.render_create_dialog(ctx, &state);
        }
    }
}

impl MiaoApp {
    fn render_create_dialog(&mut self, ctx: &egui::Context, state: &AsyncState) {
        if !state.versions.is_empty()
            && state.loader_versions.is_empty()
            && !state.loading_loader_versions
        {
            let mc_ver = state.versions[self.new_instance_version_idx].id.clone();
            self.fetch_loader_versions_for_version(&mc_ver, ctx);
        }

        let mut open = true;
        egui::Window::new("New Instance")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.new_instance_name);
                });

                ui.horizontal(|ui| {
                    ui.label("MC Version:");
                    if !state.versions.is_empty() {
                        let current = state
                            .versions
                            .get(self.new_instance_version_idx)
                            .map(|v| v.id.as_str())
                            .unwrap_or("?");
                        let prev_version_idx = self.new_instance_version_idx;
                        egui::ComboBox::from_id_salt("mc_ver")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (i, ver) in state.versions.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.new_instance_version_idx,
                                        i,
                                        &ver.id,
                                    );
                                }
                            });
                        if self.new_instance_version_idx != prev_version_idx {
                            self.new_instance_loader_version_idx = 0;
                            self.new_instance_loader = 0;
                            let mc_ver = state.versions[self.new_instance_version_idx].id.clone();
                            self.fetch_loader_versions_for_version(&mc_ver, ctx);
                        }
                    } else {
                        ui.label("Loading...");
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Mod Loader:");
                    let available_loaders = self.get_available_loaders(state);
                    let current_loader_name = available_loaders
                        .iter()
                        .find(|(idx, _, _)| *idx == self.new_instance_loader)
                        .map(|(_, name, _)| *name)
                        .unwrap_or("None (Vanilla)");

                    egui::ComboBox::from_id_salt("loader")
                        .selected_text(current_loader_name)
                        .show_ui(ui, |ui| {
                            for (idx, name, available) in &available_loaders {
                                let label = if *available {
                                    *name
                                } else {
                                    &format!("{} (unavailable)", name)
                                };
                                ui.add_enabled_ui(*available, |ui| {
                                    ui.selectable_value(&mut self.new_instance_loader, *idx, label);
                                });
                            }
                        });
                });

                if self.new_instance_loader > 0 {
                    let loader_versions = self.get_current_loader_versions(state);
                    if !loader_versions.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Loader Version:");
                            let current_version = loader_versions
                                .get(self.new_instance_loader_version_idx)
                                .map(|v| v.version.as_str())
                                .unwrap_or("?");
                            egui::ComboBox::from_id_salt("loader_ver")
                                .selected_text(current_version)
                                .show_ui(ui, |ui| {
                                    for (i, v) in loader_versions.iter().enumerate() {
                                        let label = if v.stable {
                                            format!("{} ★", v.version)
                                        } else {
                                            v.version.clone()
                                        };
                                        ui.selectable_value(
                                            &mut self.new_instance_loader_version_idx,
                                            i,
                                            label,
                                        );
                                    }
                                });
                        });
                    }
                }

                ui.separator();

                if ui.button("Create").clicked() && !state.versions.is_empty() {
                    let ver = state.versions[self.new_instance_version_idx].clone();
                    let name = if self.new_instance_name.is_empty() {
                        ver.id.clone()
                    } else {
                        self.new_instance_name.clone()
                    };
                    let loader = if self.new_instance_loader == 0 {
                        None
                    } else {
                        let loader_versions = self.get_current_loader_versions(state);
                        let version = loader_versions
                            .get(self.new_instance_loader_version_idx)
                            .map(|v| v.version.clone())
                            .or_else(|| loader_versions.first().map(|v| v.version.clone()));

                        version.and_then(|v| {
                            ModLoaderType::from_index(self.new_instance_loader - 1)
                                .map(|lt| (lt.as_str().to_string(), v))
                        })
                    };

                    self.show_create_dialog = false;
                    self.create_instance_async_with_version(ver, name, loader);
                }
            });

        if !open {
            self.show_create_dialog = false;
        }
    }

    fn fetch_loader_versions_for_version(&mut self, mc_version: &str, ctx: &egui::Context) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.loading_loader_versions {
                return;
            }
            s.loading_loader_versions = true;
            s.loader_versions.clear();
        }

        self.status = format!("Loading loader versions for {}...", mc_version);

        let state = self.async_state.clone();
        let mc_version = mc_version.to_string();
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let http = reqwest::Client::new();
            rt.block_on(async {
                let versions =
                    miao_core::modloader::fetch_all_loader_versions(&http, &mc_version).await;
                let mut s = state.lock().unwrap();
                s.loading_loader_versions = false;
                match versions {
                    Ok(v) => {
                        s.loader_versions = v;
                    }
                    Err(e) => {
                        s.install_status = Some(format!("Failed to load loader versions: {}", e));
                    }
                }
            });
            ctx.request_repaint();
        });
    }

    fn get_available_loaders(&self, state: &AsyncState) -> Vec<(usize, &'static str, bool)> {
        let mut loaders = Vec::new();
        loaders.push((0, "None (Vanilla)", true));

        let loader_types = [
            (1, "Fabric", ModLoaderType::Fabric),
            (2, "Quilt", ModLoaderType::Quilt),
            (3, "NeoForge", ModLoaderType::NeoForge),
            (4, "Forge", ModLoaderType::Forge),
        ];

        for (idx, name, loader_type) in loader_types {
            let available = state.loader_versions.contains_key(&loader_type);
            loaders.push((idx, name, available));
        }

        loaders
    }

    fn get_current_loader_versions<'a>(&self, state: &'a AsyncState) -> Vec<&'a ModLoaderVersion> {
        if self.new_instance_loader == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.new_instance_loader - 1)
            .and_then(|lt| state.loader_versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    fn create_instance_async_with_version(
        &mut self,
        ver: VersionInfo,
        instance_name: String,
        loader: Option<(String, String)>,
    ) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.installing {
                self.status = "Already installing...".to_string();
                return;
            }
            s.installing = true;
            s.install_status = Some(format!("Creating '{}'...", instance_name));
        }

        self.status = format!("Creating '{}'...", instance_name);

        let state = self.async_state.clone();
        let config = self.config.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = do_create_instance_with_version(
                    &ver,
                    &instance_name,
                    loader.as_ref().map(|(lt, lv)| (lt.as_str(), lv.as_str())),
                    &config,
                )
                .await;
                let mut s = state.lock().unwrap();
                s.installing = false;
                match result {
                    Ok(_) => s.install_status = Some(format!("✓ '{}' created!", instance_name)),
                    Err(e) => s.install_status = Some(format!("✗ Failed: {}", e)),
                }
            });
        });
    }

    fn download_java_for_instance(&mut self, idx: usize) {
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

        let meta_content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(_) => {
                self.status = "Cannot read version meta.".to_string();
                return;
            }
        };

        let meta: miao_core::version::meta::VersionMeta = match serde_json::from_str(&meta_content)
        {
            Ok(m) => m,
            Err(_) => {
                self.status = "Cannot parse version meta.".to_string();
                return;
            }
        };

        let required = meta.required_java_major();
        let java_installations = miao_core::java::detect_system_java();
        if miao_core::java::find_compatible_java(&java_installations, required).is_some() {
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
                            miao_core::java::download::download_and_extract_java(
                                &http, &asset, &java_dir,
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
}

impl MiaoApp {
    fn render_instances(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Game Instances");
            if ui.button("+ New Instance").clicked() {
                self.show_create_dialog = true;
                self.new_instance_name.clear();
                self.new_instance_version_idx = 0;
                self.new_instance_loader = 0;
            }
        });
        ui.separator();

        if self.instances.is_empty() {
            ui.label("No instances. Click '+ New Instance' to create one.");
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
            ui.horizontal(|ui| {
                if let Some(idx) = self.selected_instance
                    && ui.button("▶ Launch").clicked()
                {
                    self.launch_instance(idx);
                }

                if let Some(idx) = self.selected_instance {
                    if ui.button("🗑 Delete").clicked() {
                        let name = self.instances[idx].name.clone();
                        if let Err(e) = miao_core::instance::delete_instance(
                            &self.config.instances_dir(),
                            &name,
                        ) {
                            self.status = format!("✗ Delete failed: {}", e);
                        } else {
                            self.status = format!("✓ Deleted '{}'", name);
                            self.instances =
                                miao_core::instance::list_instances(&self.config.instances_dir())
                                    .unwrap_or_default();
                            self.selected_instance = None;
                        }
                    }

                    if ui.button("📂 Open Folder").clicked() {
                        let inst = &self.instances[idx];
                        let dir = miao_core::instance::Instance::instance_dir(
                            &self.config.instances_dir(),
                            &inst.name,
                        );
                        let _ = miao_core::instance::open_folder(&dir);
                    }

                    if ui.button("☕ Download Java").clicked() {
                        self.download_java_for_instance(idx);
                    }
                }
            });

            if let Some(idx) = self.selected_instance {
                ui.separator();
                ui.horizontal(|ui| {
                    egui::ComboBox::from_label("Mod Loader")
                        .selected_text(LOADER_NAMES[self.selected_loader])
                        .show_ui(ui, |ui| {
                            for (i, name) in LOADER_NAMES.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_loader, i, *name);
                            }
                        });

                    if ui.button("Install Loader").clicked() {
                        let inst = &self.instances[idx];
                        self.install_loader(
                            inst.name.clone(),
                            inst.minecraft_version.clone(),
                            self.selected_loader,
                        );
                    }
                });

                self.render_instance_resources(ui, idx);
            }
        }
    }

    fn render_instance_resources(&mut self, ui: &mut egui::Ui, idx: usize) {
        let inst = &self.instances[idx];
        let instance_dir =
            miao_core::instance::Instance::instance_dir(&self.config.instances_dir(), &inst.name);

        ui.separator();
        egui::CollapsingHeader::new("Mods").show(ui, |ui| {
            let mods_dir = miao_core::instance::Instance::mods_dir(&instance_dir);
            let mods = miao_core::modmanager::scan_mods_dir(&mods_dir);
            if mods.is_empty() {
                ui.label("No mods installed.");
            } else {
                for mut m in mods {
                    ui.horizontal(|ui| {
                        let status = if m.enabled { "✓" } else { "✗" };
                        if ui.button(status).clicked() {
                            let _ = m.toggle();
                        }
                        ui.label(&m.name);
                        if ui.small_button("🗑").clicked() {
                            let _ = m.delete();
                        }
                    });
                }
            }
            if ui.button("Open mods folder").clicked() {
                let _ = miao_core::instance::open_folder(&mods_dir);
            }
        });

        egui::CollapsingHeader::new("Resource Packs").show(ui, |ui| {
            let dir = miao_core::instance::Instance::resourcepacks_dir(&instance_dir);
            let packs = miao_core::resource::scan_resourcepacks(&dir);
            if packs.is_empty() {
                ui.label("No resource packs.");
            } else {
                for p in &packs {
                    ui.horizontal(|ui| {
                        ui.label(&p.name);
                        if ui.small_button("🗑").clicked() {
                            let _ = p.delete();
                        }
                    });
                }
            }
            if ui.button("Open folder").clicked() {
                let _ = miao_core::instance::open_folder(&dir);
            }
        });

        egui::CollapsingHeader::new("Shaders").show(ui, |ui| {
            let dir = miao_core::instance::Instance::shaderpacks_dir(&instance_dir);
            let shaders = miao_core::resource::scan_shaderpacks(&dir);
            if shaders.is_empty() {
                ui.label("No shader packs.");
            } else {
                for s in &shaders {
                    ui.horizontal(|ui| {
                        ui.label(&s.name);
                        if ui.small_button("🗑").clicked() {
                            let _ = s.delete();
                        }
                    });
                }
            }
            if ui.button("Open folder").clicked() {
                let _ = miao_core::instance::open_folder(&dir);
            }
        });

        egui::CollapsingHeader::new("Worlds").show(ui, |ui| {
            let saves = miao_core::instance::list_saves(&instance_dir);
            if saves.is_empty() {
                ui.label("No worlds.");
            } else {
                for s in &saves {
                    ui.horizontal(|ui| {
                        ui.label(&s.name);
                        if ui.small_button("🗑").clicked() {
                            let _ = s.delete();
                        }
                    });
                }
            }
            let saves_dir = miao_core::instance::Instance::saves_dir(&instance_dir);
            if ui.button("Open folder").clicked() {
                let _ = miao_core::instance::open_folder(&saves_dir);
            }
        });
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

async fn do_loader_install(
    instance_name: &str,
    mc_version: &str,
    loader_idx: usize,
    config: &LauncherConfig,
) -> anyhow::Result<String> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

    let http = reqwest::Client::new();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);

    let (loader_type, loader_ver) = match loader_idx {
        0 => {
            let versions = fabric::fetch_loader_versions(&http, mc_version).await?;
            let ver = versions
                .iter()
                .find(|v| v.loader.stable)
                .or(versions.first())
                .map(|v| v.loader.version.clone())
                .ok_or_else(|| anyhow::anyhow!("No Fabric versions for {}", mc_version))?;
            let profile = fabric::fetch_profile(&http, mc_version, &ver).await?;
            let tasks = fabric::collect_fabric_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            (ModLoaderType::Fabric, ver)
        }
        1 => {
            let versions = quilt::fetch_loader_versions(&http, mc_version).await?;
            let ver = versions
                .first()
                .map(|v| v.loader.version.clone())
                .ok_or_else(|| anyhow::anyhow!("No Quilt versions for {}", mc_version))?;
            let profile = quilt::fetch_profile(&http, mc_version, &ver).await?;
            let tasks = quilt::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            (ModLoaderType::Quilt, ver)
        }
        2 => {
            let versions = neoforge::fetch_versions(&http, mc_version).await?;
            let ver = versions
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No NeoForge versions for {}", mc_version))?;
            let profile = neoforge::fetch_profile(&http, &ver).await?;
            let tasks = neoforge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            (ModLoaderType::NeoForge, ver)
        }
        3 => {
            let ver = forge::fetch_recommended_version(&http, mc_version)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No Forge versions for {}", mc_version))?;
            let profile = forge::fetch_install_profile(&http, mc_version, &ver).await?;
            let tasks = forge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            (ModLoaderType::Forge, ver)
        }
        _ => anyhow::bail!("Invalid loader"),
    };

    let mut inst = Instance::load_from(&instance_dir)?;
    inst.mod_loader = Some(miao_core::instance::ModLoaderConfig {
        loader_type: loader_type.clone(),
        version: loader_ver.clone(),
    });
    inst.save_to(&instance_dir)?;

    Ok(format!(
        "{} {} for '{}'",
        loader_type, loader_ver, instance_name
    ))
}

async fn do_create_instance_with_version(
    ver: &VersionInfo,
    instance_name: &str,
    loader: Option<(&str, &str)>,
    config: &LauncherConfig,
) -> anyhow::Result<()> {
    use miao_core::modloader::ModLoaderType;

    let http = reqwest::Client::new();

    let meta =
        miao_core::version::install::fetch_version_meta(&http, &ver.url, &config.download_mirror)
            .await?;

    miao_core::version::install::save_version_meta(&meta, config)?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, config, &config.download_mirror);
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
        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks).await?;
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
