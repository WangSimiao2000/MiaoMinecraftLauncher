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
    mod_search_hits: Option<Vec<miao_core::modrinth::api::SearchHit>>,
    mod_versions: Option<Vec<miao_core::modrinth::api::ProjectVersion>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dialog {
    None,
    NewInstance,
    Accounts,
    Settings,
    ModSearch,
}

pub struct MiaoApp {
    config: LauncherConfig,
    instances: Vec<Instance>,
    async_state: Arc<Mutex<AsyncState>>,
    active_dialog: Dialog,
    selected_instance: Option<usize>,
    status: String,
    offline_username_input: String,
    new_instance_name: String,
    new_instance_version_idx: usize,
    new_instance_loader: usize,
    new_instance_loader_version_idx: usize,

    mod_search_query: String,
    mod_search_results: Vec<miao_core::modrinth::api::SearchHit>,
    mod_search_versions: Vec<miao_core::modrinth::api::ProjectVersion>,
    mod_search_selected: usize,
    mod_searching: bool,
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
            active_dialog: Dialog::None,
            selected_instance: None,
            status: "Ready".to_string(),
            offline_username_input: String::new(),
            new_instance_name: String::new(),
            new_instance_version_idx: 0,
            new_instance_loader: 0,
            new_instance_loader_version_idx: 0,

            mod_search_query: String::new(),
            mod_search_results: Vec::new(),
            mod_search_versions: Vec::new(),
            mod_search_selected: 0,
            mod_searching: false,
        }
    }
}

impl eframe::App for MiaoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = self.async_state.lock().unwrap().clone();

        if let Some(ref s) = state.install_status {
            self.status = s.clone();
            if s.starts_with('✓') {
                self.instances =
                    instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
                self.async_state.lock().unwrap().install_status = None;
            } else if s.starts_with('✗') || (!state.installing && !state.ms_logging_in) {
                self.async_state.lock().unwrap().install_status = None;
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

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🐱 MiaoMC");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙ Settings").clicked() {
                        self.active_dialog = Dialog::Settings;
                    }
                    if ui.button("👤 Accounts").clicked() {
                        self.active_dialog = Dialog::Accounts;
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label(&self.status);
        });

        egui::SidePanel::left("instance_list")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Instances");
                    if ui.button("+ New").clicked() {
                        self.active_dialog = Dialog::NewInstance;
                        self.new_instance_name.clear();
                        self.new_instance_version_idx = 0;
                        self.new_instance_loader = 0;
                        self.new_instance_loader_version_idx = 0;
                    }
                    if ui.button("Import").clicked() {
                        self.import_with_dialog();
                    }
                });
                ui.separator();

                if self.instances.is_empty() {
                    ui.label("No instances yet.");
                } else {
                    for (i, inst) in self.instances.iter().enumerate() {
                        let selected = self.selected_instance == Some(i);
                        let label = format!(
                            "{}{}",
                            inst.name,
                            inst.mod_loader
                                .as_ref()
                                .map(|l| format!(" [{}]", l.loader_type))
                                .unwrap_or_default()
                        );
                        if ui.selectable_label(selected, &label).clicked() {
                            self.selected_instance = Some(i);
                        }
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_detail(ui);
        });

        match self.active_dialog {
            Dialog::NewInstance => self.render_new_instance_dialog(ctx, &state),
            Dialog::Accounts => self.render_accounts_dialog(ctx, &state),
            Dialog::Settings => self.render_settings_dialog(ctx),
            Dialog::ModSearch => self.render_mod_search_dialog(ctx),
            Dialog::None => {}
        }
    }
}

impl MiaoApp {
    fn render_detail(&mut self, ui: &mut egui::Ui) {
        let Some(idx) = self.selected_instance else {
            ui.centered_and_justified(|ui| {
                ui.label("Select an instance or click '+ New' to create one.");
            });
            return;
        };

        let inst = self.instances[idx].clone();

        ui.heading(&inst.name);
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(format!("MC: {}", inst.minecraft_version));
            ui.separator();
            let loader = inst
                .mod_loader
                .as_ref()
                .map(|l| format!("{} {}", l.loader_type, l.version))
                .unwrap_or_else(|| "Vanilla".to_string());
            ui.label(format!("Loader: {}", loader));
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("▶ Launch").clicked() {
                self.launch_instance(idx);
            }
            if ui.button("🗑 Delete").clicked() {
                let name = inst.name.clone();
                if let Err(e) =
                    miao_core::instance::delete_instance(&self.config.instances_dir(), &name)
                {
                    self.status = format!("✗ Delete failed: {}", e);
                } else {
                    self.status = format!("✓ Deleted '{}'", name);
                    self.instances =
                        instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
                    self.selected_instance = None;
                }
            }
            if ui.button("📂 Open").clicked() {
                let dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
                let _ = instance::open_folder(&dir);
            }
            if ui.button("☕ Java").clicked() {
                self.download_java_for_instance(idx);
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button("🔍 Search Mods").clicked() {
                self.active_dialog = Dialog::ModSearch;
                self.mod_search_query.clear();
                self.mod_search_results.clear();
                self.mod_search_versions.clear();
            }
            if ui.button("📦 Export .mrpack").clicked() {
                self.export_instance_with_dialog(idx);
            }
        });

        ui.add_space(12.0);

        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);

        egui::CollapsingHeader::new("Mods")
            .default_open(true)
            .show(ui, |ui| {
                let mods_dir = Instance::mods_dir(&instance_dir);
                let mods = miao_core::modmanager::scan_mods_dir(&mods_dir);
                if mods.is_empty() {
                    ui.label("No mods.");
                } else {
                    for mut m in mods {
                        ui.horizontal(|ui| {
                            if m.enabled {
                                let btn = egui::Button::new(
                                    egui::RichText::new("ON").color(egui::Color32::GREEN),
                                );
                                if ui.add(btn).clicked() {
                                    let _ = m.toggle();
                                }
                                ui.label(&m.name);
                            } else {
                                let btn = egui::Button::new(
                                    egui::RichText::new("OFF").color(egui::Color32::GRAY),
                                );
                                if ui.add(btn).clicked() {
                                    let _ = m.toggle();
                                }
                                ui.label(
                                    egui::RichText::new(&m.name).color(egui::Color32::DARK_GRAY),
                                );
                            }
                            if ui.small_button("Del").clicked() {
                                let _ = m.delete();
                            }
                        });
                    }
                }
                if ui.small_button("Open folder").clicked() {
                    let _ = instance::open_folder(&mods_dir);
                }
            });

        egui::CollapsingHeader::new("Resource Packs").show(ui, |ui| {
            let dir = Instance::resourcepacks_dir(&instance_dir);
            let packs = miao_core::resource::scan_resourcepacks(&dir);
            if packs.is_empty() {
                ui.label("None.");
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
            if ui.small_button("Open folder").clicked() {
                let _ = instance::open_folder(&dir);
            }
        });

        egui::CollapsingHeader::new("Shaders").show(ui, |ui| {
            let dir = Instance::shaderpacks_dir(&instance_dir);
            let shaders = miao_core::resource::scan_shaderpacks(&dir);
            if shaders.is_empty() {
                ui.label("None.");
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
            if ui.small_button("Open folder").clicked() {
                let _ = instance::open_folder(&dir);
            }
        });

        egui::CollapsingHeader::new("Worlds").show(ui, |ui| {
            let saves = instance::list_saves(&instance_dir);
            if saves.is_empty() {
                ui.label("None.");
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
            let saves_dir = Instance::saves_dir(&instance_dir);
            if ui.small_button("Open folder").clicked() {
                let _ = instance::open_folder(&saves_dir);
            }
        });
    }

    fn render_new_instance_dialog(&mut self, ctx: &egui::Context, state: &AsyncState) {
        if !state.versions.is_empty()
            && state.loader_versions.is_empty()
            && !state.loading_loader_versions
        {
            let mc_ver = state.versions[self.new_instance_version_idx].id.clone();
            self.fetch_loader_versions(ctx, &mc_ver);
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
                        let prev_idx = self.new_instance_version_idx;
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
                        if self.new_instance_version_idx != prev_idx {
                            self.new_instance_loader = 0;
                            self.new_instance_loader_version_idx = 0;
                            let mc_ver = state.versions[self.new_instance_version_idx].id.clone();
                            self.fetch_loader_versions(ctx, &mc_ver);
                        }
                    } else {
                        ui.label("Loading...");
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Mod Loader:");
                    let loaders = self.get_available_loaders(state);
                    let current_name = loaders
                        .iter()
                        .find(|(idx, _, _)| *idx == self.new_instance_loader)
                        .map(|(_, name, _)| *name)
                        .unwrap_or("None (Vanilla)");

                    egui::ComboBox::from_id_salt("loader")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for (idx, name, available) in &loaders {
                                ui.add_enabled_ui(*available, |ui| {
                                    let label = if *available {
                                        name.to_string()
                                    } else {
                                        format!("{} (N/A)", name)
                                    };
                                    ui.selectable_value(&mut self.new_instance_loader, *idx, label);
                                });
                            }
                        });
                });

                if self.new_instance_loader > 0 {
                    let loader_versions = self.get_loader_versions(state);
                    if !loader_versions.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Version:");
                            let current = loader_versions
                                .get(self.new_instance_loader_version_idx)
                                .map(|v| v.version.as_str())
                                .unwrap_or("?");
                            egui::ComboBox::from_id_salt("loader_ver")
                                .selected_text(current)
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
                        let versions = self.get_loader_versions(state);
                        versions
                            .get(self.new_instance_loader_version_idx)
                            .or(versions.first())
                            .and_then(|v| {
                                ModLoaderType::from_index(self.new_instance_loader - 1)
                                    .map(|lt| (lt.as_str().to_string(), v.version.clone()))
                            })
                    };

                    self.active_dialog = Dialog::None;
                    self.create_instance(ver, name, loader);
                }
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }

    fn render_accounts_dialog(&mut self, ctx: &egui::Context, state: &AsyncState) {
        let mut open = true;
        egui::Window::new("Accounts")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                if self.config.accounts.is_empty() {
                    ui.label("No accounts.");
                } else {
                    for (i, acc) in self.config.accounts.iter().enumerate() {
                        let active = self.config.active_account_index == Some(i);
                        let marker = if active { "★" } else { " " };
                        let acc_type = if acc.is_microsoft() { "MS" } else { "Offline" };
                        ui.label(format!("{} [{}] {}", marker, acc_type, acc.username()));
                    }
                }

                ui.separator();
                ui.label("Add offline account:");
                ui.horizontal(|ui| {
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
                if let Some(ref dc) = state.ms_device_code {
                    ui.label(format!("Go to: {}", dc.verification_uri));
                    ui.label(format!("Code: {}", dc.user_code));
                    ui.label("Waiting...");
                } else if state.ms_logging_in {
                    ui.label("Connecting...");
                } else if ui.button("Microsoft Login").clicked() {
                    self.start_ms_login(ctx.clone());
                }
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }

    fn render_settings_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!("Data: {}", self.config.data_dir.display()));
                ui.label(format!("Mirror: {:?}", self.config.download_mirror));
                ui.label(format!(
                    "Concurrent downloads: {}",
                    self.config.max_concurrent_downloads
                ));
                ui.separator();
                ui.label("Java installations:");
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
            });

        if !open {
            self.active_dialog = Dialog::None;
        }
    }
}

impl MiaoApp {
    fn get_available_loaders(&self, state: &AsyncState) -> Vec<(usize, &'static str, bool)> {
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

    fn get_loader_versions<'a>(&self, state: &'a AsyncState) -> Vec<&'a ModLoaderVersion> {
        if self.new_instance_loader == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.new_instance_loader - 1)
            .and_then(|lt| state.loader_versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    fn fetch_loader_versions(&mut self, ctx: &egui::Context, mc_version: &str) {
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
        let ctx = ctx.clone();

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
                "No Java {} found! Click '☕ Java' to install.",
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

    fn create_instance(
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

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = do_create_instance(
                    &ver,
                    &name,
                    loader.as_ref().map(|(lt, lv)| (lt.as_str(), lv.as_str())),
                    &config,
                )
                .await;
                let mut s = state.lock().unwrap();
                s.installing = false;
                match result {
                    Ok(_) => s.install_status = Some(format!("✓ '{}' created!", name)),
                    Err(e) => s.install_status = Some(format!("✗ Failed: {}", e)),
                }
            });
        });
    }

    fn start_ms_login(&mut self, ctx: egui::Context) {
        {
            let mut s = self.async_state.lock().unwrap();
            if s.ms_logging_in {
                return;
            }
            s.ms_logging_in = true;
        }

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

    fn render_mod_search_dialog(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut do_search = false;
        let mut do_install: Option<usize> = None;
        let mut do_load_versions: Option<usize> = None;

        egui::Window::new("Modrinth Mod Search")
            .open(&mut open)
            .resizable(true)
            .default_width(500.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    let response = ui.text_edit_singleline(&mut self.mod_search_query);
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        do_search = true;
                    }
                    if ui.button("Search").clicked() {
                        do_search = true;
                    }
                });

                if self.mod_searching {
                    ui.spinner();
                    ui.label("Searching...");
                }

                if !self.mod_search_results.is_empty() && self.mod_search_versions.is_empty() {
                    ui.separator();
                    ui.label("Results (click to select):");
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (i, hit) in self.mod_search_results.iter().enumerate() {
                                let dl = format_downloads_gui(hit.downloads);
                                let label =
                                    format!("{} — {} — ↓{}", hit.title, hit.description, dl);
                                let truncated = if label.chars().count() > 80 {
                                    let s: String = label.chars().take(79).collect();
                                    format!("{}…", s)
                                } else {
                                    label
                                };
                                if ui.selectable_label(false, &truncated).clicked() {
                                    do_load_versions = Some(i);
                                }
                            }
                        });
                }

                if !self.mod_search_versions.is_empty() {
                    ui.separator();
                    let hit_title = self
                        .mod_search_results
                        .get(self.mod_search_selected)
                        .map(|h| h.title.as_str())
                        .unwrap_or("?");
                    ui.label(format!("Versions for '{}':", hit_title));
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for (i, ver) in self.mod_search_versions.iter().enumerate().take(15) {
                                let label = format!("{} [{}]", ver.name, ver.version_type);
                                if ui.button(&label).clicked() {
                                    do_install = Some(i);
                                }
                            }
                        });
                    if ui.button("← Back to results").clicked() {
                        self.mod_search_versions.clear();
                    }
                }
            });

        if !open {
            self.active_dialog = Dialog::None;
        }

        if do_search && !self.mod_search_query.is_empty() {
            self.do_mod_search(ctx);
        }

        if let Some(idx) = do_load_versions {
            self.mod_search_selected = idx;
            self.load_mod_versions(ctx, idx);
        }

        if let Some(idx) = do_install {
            self.install_mod_version(ctx, idx);
        }
    }

    fn do_mod_search(&mut self, ctx: &egui::Context) {
        let Some(inst_idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[inst_idx];
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string());
        let query = self.mod_search_query.clone();
        let state = self.async_state.clone();
        let ctx = ctx.clone();
        self.mod_searching = true;
        self.mod_search_results.clear();
        self.mod_search_versions.clear();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = miao_core::modrinth::api::search_mods(
                    &query,
                    Some(&mc_version),
                    loader.as_deref(),
                    20,
                )
                .await;
                let mut s = state.lock().unwrap();
                match result {
                    Ok(r) => {
                        s.mod_search_hits = Some(r.hits);
                    }
                    Err(e) => {
                        s.install_status = Some(format!("✗ Search failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    fn load_mod_versions(&mut self, ctx: &egui::Context, hit_idx: usize) {
        let Some(hit) = self.mod_search_results.get(hit_idx) else {
            return;
        };
        let Some(inst_idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[inst_idx];
        let project_slug = hit.slug.clone();
        let mc_version = inst.minecraft_version.clone();
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str().to_string());
        let state = self.async_state.clone();
        let ctx = ctx.clone();
        self.mod_searching = true;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let result = miao_core::modrinth::api::get_project_versions(
                    &project_slug,
                    Some(&mc_version),
                    loader.as_deref(),
                )
                .await;
                let mut s = state.lock().unwrap();
                match result {
                    Ok(versions) => {
                        s.mod_versions = Some(versions);
                    }
                    Err(e) => {
                        s.install_status = Some(format!("✗ {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    fn install_mod_version(&mut self, ctx: &egui::Context, ver_idx: usize) {
        let Some(version) = self.mod_search_versions.get(ver_idx) else {
            return;
        };
        let Some(inst_idx) = self.selected_instance else {
            return;
        };
        let inst = &self.instances[inst_idx];
        let file = match version
            .files
            .iter()
            .find(|f| f.primary)
            .or(version.files.first())
        {
            Some(f) => f.clone(),
            None => {
                self.status = "No files in version.".to_string();
                return;
            }
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let mods_dir = Instance::mods_dir(&instance_dir);
        let state = self.async_state.clone();
        let ctx = ctx.clone();
        let version_name = version.name.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                match miao_core::modrinth::api::download_mod_file(&file, &mods_dir).await {
                    Ok(dest) => {
                        let mut s = state.lock().unwrap();
                        s.install_status = Some(format!(
                            "✓ Installed {} ({})",
                            version_name,
                            dest.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.install_status = Some(format!("✗ Install failed: {}", e));
                    }
                }
                ctx.request_repaint();
            });
        });
    }

    fn export_instance_with_dialog(&mut self, idx: usize) {
        let inst = self.instances[idx].clone();
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let state = self.async_state.clone();

        let folder = rfd::FileDialog::new()
            .set_title("Export .mrpack — select output folder")
            .pick_folder();

        let Some(output_path) = folder else {
            return;
        };

        self.status = format!("Exporting '{}'...", inst.name);
        std::thread::spawn(move || {
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
        });
    }

    fn import_with_dialog(&mut self) {
        let file = rfd::FileDialog::new()
            .set_title("Import .mrpack")
            .add_filter("Modrinth Modpack", &["mrpack"])
            .pick_file();

        let Some(mrpack_path) = file else {
            return;
        };

        let config = self.config.clone();
        let state = self.async_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.installing = true;
            s.install_status = Some(format!("Importing {}...", mrpack_path.display()));
        }

        std::thread::spawn(move || {
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
            });
        });
    }
}

fn format_downloads_gui(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

async fn do_create_instance(
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
