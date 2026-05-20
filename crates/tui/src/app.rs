use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{DeviceCodeResponse, MicrosoftAuth, PollResult};
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance, SaveWorld};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::modmanager::ModInfo;
use miao_core::resource::{ResourcePack, ShaderPack};
use miao_core::version::VersionInfo;
use std::collections::HashMap;
use tokio::sync::mpsc;

const MS_CLIENT_ID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Instances,
    Versions,
    Accounts,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Instances, Tab::Versions, Tab::Accounts, Tab::Settings];

    pub fn title(&self) -> &str {
        match self {
            Self::Instances => "Instances",
            Self::Versions => "Versions",
            Self::Accounts => "Accounts",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Input,
    SelectLoader,
    CreateName,
    CreateSelectVersion,
    CreateSelectLoader,
    ManageInstance,
    ManageMods,
    ManageResourcePacks,
    ManageShaders,
    ManageSaves,
    ConfirmDelete,
}

pub const LOADER_OPTIONS: [&str; 5] = ["None (Vanilla)", "Fabric", "Quilt", "NeoForge", "Forge"];

#[derive(Debug, Clone)]
pub enum AsyncMessage {
    VersionsLoaded(Vec<VersionInfo>),
    LoaderVersionsLoaded(String, HashMap<ModLoaderType, Vec<ModLoaderVersion>>),
    InstallProgress(String),
    InstallDone(String),
    InstallError(String),
    MsDeviceCode(DeviceCodeResponse),
    MsLoginSuccess(String),
    MsLoginError(String),
    JavaDownloadDone(String),
}

pub struct App {
    pub config: LauncherConfig,
    pub current_tab: usize,
    pub selected_index: usize,
    pub instances: Vec<Instance>,
    pub versions: Vec<VersionInfo>,
    pub status_message: String,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub loading: bool,
    pub installing: bool,
    pub show_snapshots: bool,
    pub loader_cursor: usize,
    pub ms_device_code: Option<DeviceCodeResponse>,
    pub rx: mpsc::UnboundedReceiver<AsyncMessage>,
    pub tx: mpsc::UnboundedSender<AsyncMessage>,
    pub all_versions: Vec<VersionInfo>,
    pub create_name: String,
    pub create_version_cursor: usize,
    pub create_loader_cursor: usize,
    pub loader_versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    pub loader_version_cursor: usize,
    pub loading_loader_versions: bool,
    pub manage_mods: Vec<ModInfo>,
    pub manage_resourcepacks: Vec<ResourcePack>,
    pub manage_shaders: Vec<ShaderPack>,
    pub manage_saves: Vec<SaveWorld>,
    pub manage_cursor: usize,
}

impl App {
    pub fn new() -> anyhow::Result<Self> {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();
        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            current_tab: 0,
            selected_index: 0,
            instances,
            versions: Vec::new(),
            status_message: "[q]uit [Tab]switch [j/k]nav [i]nstall [l]aunch [f]loader [a]ccount [m]s [s]napshots"
                .to_string(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            loading: true,
            installing: false,
            show_snapshots: false,
            loader_cursor: 0,
            ms_device_code: None,
            rx,
            tx,
            all_versions: Vec::new(),
            create_name: String::new(),
            create_version_cursor: 0,
            create_loader_cursor: 0,
            loader_versions: HashMap::new(),
            loader_version_cursor: 0,
            loading_loader_versions: false,
            manage_mods: Vec::new(),
            manage_resourcepacks: Vec::new(),
            manage_shaders: Vec::new(),
            manage_saves: Vec::new(),
            manage_cursor: 0,
        })
    }

    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncMessage::VersionsLoaded(v) => {
                    self.all_versions = v;
                    self.filter_versions();
                    self.loading = false;
                }
                AsyncMessage::LoaderVersionsLoaded(mc_version, versions) => {
                    self.loader_versions = versions;
                    self.loading_loader_versions = false;
                    self.status_message = format!("Loaded loader versions for {}", mc_version);
                }
                AsyncMessage::InstallProgress(s) => {
                    self.status_message = s;
                }
                AsyncMessage::InstallDone(name) => {
                    self.installing = false;
                    self.status_message = format!("✓ Installed '{}'! Press 'r' to refresh.", name);
                    self.refresh_instances();
                }
                AsyncMessage::InstallError(e) => {
                    self.installing = false;
                    self.status_message = format!("✗ Install failed: {}", e);
                }
                AsyncMessage::MsDeviceCode(dc) => {
                    self.ms_device_code = Some(dc);
                }
                AsyncMessage::MsLoginSuccess(name) => {
                    self.ms_device_code = None;
                    self.status_message = format!("✓ Logged in as {}", name);
                    self.config = LauncherConfig::load().unwrap_or_default();
                }
                AsyncMessage::MsLoginError(e) => {
                    self.ms_device_code = None;
                    self.status_message = format!("✗ Login failed: {}", e);
                }
                AsyncMessage::JavaDownloadDone(msg) => {
                    self.installing = false;
                    self.status_message = msg;
                }
            }
        }
    }

    pub fn active_tab(&self) -> Tab {
        Tab::ALL[self.current_tab]
    }

    pub fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % Tab::ALL.len();
        self.selected_index = 0;
    }

    pub fn prev_tab(&mut self) {
        self.current_tab = if self.current_tab == 0 {
            Tab::ALL.len() - 1
        } else {
            self.current_tab - 1
        };
        self.selected_index = 0;
    }

    pub fn next_item(&mut self) {
        let max = self.current_list_len();
        if max > 0 {
            self.selected_index = (self.selected_index + 1) % max;
        }
    }

    pub fn prev_item(&mut self) {
        let max = self.current_list_len();
        if max > 0 {
            self.selected_index = if self.selected_index == 0 {
                max - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn select_item(&mut self) {
        match self.active_tab() {
            Tab::Instances => {
                if let Some(inst) = self.instances.get(self.selected_index) {
                    self.status_message = format!("'{}' selected. Press 'l' to launch.", inst.name);
                }
            }
            Tab::Versions => {
                if let Some(ver) = self.versions.get(self.selected_index) {
                    self.status_message = format!("MC {} selected. Press 'i' to install.", ver.id);
                }
            }
            _ => {}
        }
    }

    pub fn go_back(&mut self) {
        if self.input_mode == InputMode::Input {
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
            self.status_message = "Cancelled.".to_string();
        }
    }

    pub fn start_add_account(&mut self) {
        self.input_mode = InputMode::Input;
        self.input_buffer.clear();
        self.status_message = "Enter username (Enter=confirm, Esc=cancel):".to_string();
    }

    pub fn confirm_input(&mut self) {
        if self.input_mode == InputMode::Input && !self.input_buffer.is_empty() {
            let username = self.input_buffer.clone();
            let account = create_offline_account(&username);
            self.status_message = format!("✓ Added offline account: {}", account.username);
            self.config.accounts.push(AuthMethod::Offline(account));
            if self.config.active_account_index.is_none() {
                self.config.active_account_index = Some(0);
            }
            let _ = self.config.save();
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
        }
    }

    pub fn launch_selected(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            self.status_message = "No instance selected.".to_string();
            return;
        };

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
            self.status_message = format!(
                "Version meta not found. Install {} first.",
                inst.minecraft_version
            );
            return;
        }

        let meta_content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(e) => {
                self.status_message = format!("Error reading meta: {}", e);
                return;
            }
        };

        let meta: miao_core::version::meta::VersionMeta = match serde_json::from_str(&meta_content)
        {
            Ok(m) => m,
            Err(e) => {
                self.status_message = format!("Error parsing meta: {}", e);
                return;
            }
        };

        let required_java = meta.required_java_major();
        let java_path = inst.java_path.clone().or_else(|| {
            java::find_compatible_java(&java_installations, required_java).map(|j| j.path.clone())
        });

        let Some(java_path) = java_path else {
            self.status_message = format!(
                "No Java {} found! Press 'e' → 'j' to download, or install manually.",
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
                Ok(_) => {
                    self.status_message =
                        format!("✓ Launched {} with {}", inst.name, java_path.display());
                }
                Err(e) => {
                    self.status_message = format!("✗ Launch failed: {}", e);
                }
            },
            Err(e) => {
                self.status_message = format!("✗ Build command failed: {}", e);
            }
        }
    }

    pub fn start_ms_login(&mut self) {
        let tx = self.tx.clone();

        self.status_message = "Starting Microsoft login...".to_string();

        let mut config = self.config.clone();

        tokio::spawn(async move {
            let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());

            let device_code = match auth.request_device_code().await {
                Ok(dc) => dc,
                Err(e) => {
                    let _ = tx.send(AsyncMessage::MsLoginError(e.to_string()));
                    return;
                }
            };

            let _ = open::that(&device_code.verification_uri);
            let code = device_code.device_code.clone();
            let interval = device_code.interval;
            let _ = tx.send(AsyncMessage::MsDeviceCode(device_code));

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
                                let _ = tx.send(AsyncMessage::MsLoginSuccess(name));
                            }
                            Err(e) => {
                                let _ = tx.send(AsyncMessage::MsLoginError(e.to_string()));
                            }
                        }
                        return;
                    }
                    Ok(PollResult::Pending) | Ok(PollResult::SlowDown) => continue,
                    Ok(PollResult::Expired) => {
                        let _ = tx.send(AsyncMessage::MsLoginError("Code expired.".to_string()));
                        return;
                    }
                    Ok(PollResult::Error(e)) => {
                        let _ = tx.send(AsyncMessage::MsLoginError(e));
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(AsyncMessage::MsLoginError(e.to_string()));
                        return;
                    }
                }
            }
        });
    }

    pub fn refresh_instances(&mut self) {
        self.instances = instance::list_instances(&self.config.instances_dir()).unwrap_or_default();
    }

    pub fn toggle_snapshots(&mut self) {
        self.show_snapshots = !self.show_snapshots;
        self.selected_index = 0;
        self.filter_versions();
        self.status_message = if self.show_snapshots {
            "Showing all versions (including snapshots)".to_string()
        } else {
            "Showing releases only".to_string()
        };
    }

    fn filter_versions(&mut self) {
        self.versions = if self.show_snapshots {
            self.all_versions.iter().take(80).cloned().collect()
        } else {
            self.all_versions
                .iter()
                .filter(|v| v.is_release())
                .take(50)
                .cloned()
                .collect()
        };
    }

    pub fn start_manage_instance(&mut self) {
        if self.instances.is_empty() {
            self.status_message = "No instance selected.".to_string();
            return;
        }
        self.input_mode = InputMode::ManageInstance;
        self.status_message =
            "[d]elete [m]ods [r]esourcepacks [s]haders [w]orlds [o]pen folder [Esc]back"
                .to_string();
    }

    pub fn confirm_delete_instance(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            self.input_mode = InputMode::Normal;
            return;
        };
        let name = inst.name.clone();
        if let Err(e) = instance::delete_instance(&self.config.instances_dir(), &name) {
            self.status_message = format!("✗ Delete failed: {}", e);
        } else {
            self.status_message = format!("✓ Deleted '{}'", name);
            self.refresh_instances();
            self.selected_index = self
                .selected_index
                .min(self.instances.len().saturating_sub(1));
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn open_manage_mods(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let mods_dir = Instance::mods_dir(&instance_dir);
        self.manage_mods = miao_core::modmanager::scan_mods_dir(&mods_dir);
        self.manage_cursor = 0;
        self.input_mode = InputMode::ManageMods;
        self.status_message = "[j/k]nav [Enter]toggle [d]elete [o]pen folder [Esc]back".to_string();
    }

    pub fn open_manage_resourcepacks(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let dir = Instance::resourcepacks_dir(&instance_dir);
        self.manage_resourcepacks = miao_core::resource::scan_resourcepacks(&dir);
        self.manage_cursor = 0;
        self.input_mode = InputMode::ManageResourcePacks;
        self.status_message = "[j/k]nav [d]elete [o]pen folder [Esc]back".to_string();
    }

    pub fn open_manage_shaders(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let dir = Instance::shaderpacks_dir(&instance_dir);
        self.manage_shaders = miao_core::resource::scan_shaderpacks(&dir);
        self.manage_cursor = 0;
        self.input_mode = InputMode::ManageShaders;
        self.status_message = "[j/k]nav [d]elete [o]pen folder [Esc]back".to_string();
    }

    pub fn open_manage_saves(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        self.manage_saves = instance::list_saves(&instance_dir);
        self.manage_cursor = 0;
        self.input_mode = InputMode::ManageSaves;
        self.status_message = "[j/k]nav [d]elete [o]pen folder [Esc]back".to_string();
    }

    pub fn open_instance_folder(&self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), &inst.name);
        let _ = instance::open_folder(&instance_dir);
    }

    pub fn toggle_current_mod(&mut self) {
        if let Some(m) = self.manage_mods.get_mut(self.manage_cursor)
            && let Err(e) = m.toggle()
        {
            self.status_message = format!("✗ Toggle failed: {}", e);
        }
    }

    pub fn delete_current_resource(&mut self) {
        match self.input_mode {
            InputMode::ManageMods => {
                if let Some(m) = self.manage_mods.get(self.manage_cursor) {
                    let name = m.name.clone();
                    if let Err(e) = m.delete() {
                        self.status_message = format!("✗ Delete failed: {}", e);
                    } else {
                        self.status_message = format!("✓ Deleted mod '{}'", name);
                        self.manage_mods.remove(self.manage_cursor);
                        if self.manage_cursor > 0 && self.manage_cursor >= self.manage_mods.len() {
                            self.manage_cursor -= 1;
                        }
                    }
                }
            }
            InputMode::ManageResourcePacks => {
                if let Some(r) = self.manage_resourcepacks.get(self.manage_cursor) {
                    let name = r.name.clone();
                    if let Err(e) = r.delete() {
                        self.status_message = format!("✗ Delete failed: {}", e);
                    } else {
                        self.status_message = format!("✓ Deleted '{}'", name);
                        self.manage_resourcepacks.remove(self.manage_cursor);
                        if self.manage_cursor > 0
                            && self.manage_cursor >= self.manage_resourcepacks.len()
                        {
                            self.manage_cursor -= 1;
                        }
                    }
                }
            }
            InputMode::ManageShaders => {
                if let Some(s) = self.manage_shaders.get(self.manage_cursor) {
                    let name = s.name.clone();
                    if let Err(e) = s.delete() {
                        self.status_message = format!("✗ Delete failed: {}", e);
                    } else {
                        self.status_message = format!("✓ Deleted '{}'", name);
                        self.manage_shaders.remove(self.manage_cursor);
                        if self.manage_cursor > 0 && self.manage_cursor >= self.manage_shaders.len()
                        {
                            self.manage_cursor -= 1;
                        }
                    }
                }
            }
            InputMode::ManageSaves => {
                if let Some(s) = self.manage_saves.get(self.manage_cursor) {
                    let name = s.name.clone();
                    if let Err(e) = s.delete() {
                        self.status_message = format!("✗ Delete failed: {}", e);
                    } else {
                        self.status_message = format!("✓ Deleted '{}'", name);
                        self.manage_saves.remove(self.manage_cursor);
                        if self.manage_cursor > 0 && self.manage_cursor >= self.manage_saves.len() {
                            self.manage_cursor -= 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn manage_next(&mut self) {
        let max = match self.input_mode {
            InputMode::ManageMods => self.manage_mods.len(),
            InputMode::ManageResourcePacks => self.manage_resourcepacks.len(),
            InputMode::ManageShaders => self.manage_shaders.len(),
            InputMode::ManageSaves => self.manage_saves.len(),
            _ => 0,
        };
        if max > 0 {
            self.manage_cursor = (self.manage_cursor + 1) % max;
        }
    }

    pub fn manage_prev(&mut self) {
        let max = match self.input_mode {
            InputMode::ManageMods => self.manage_mods.len(),
            InputMode::ManageResourcePacks => self.manage_resourcepacks.len(),
            InputMode::ManageShaders => self.manage_shaders.len(),
            InputMode::ManageSaves => self.manage_saves.len(),
            _ => 0,
        };
        if max > 0 {
            self.manage_cursor = if self.manage_cursor == 0 {
                max - 1
            } else {
                self.manage_cursor - 1
            };
        }
    }

    pub fn download_java_for_instance(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            return;
        };

        let meta_path = self
            .config
            .versions_dir()
            .join(&inst.minecraft_version)
            .join(format!("{}.json", &inst.minecraft_version));

        if !meta_path.exists() {
            self.status_message = "Version meta not found.".to_string();
            return;
        }

        let meta_content = match std::fs::read_to_string(&meta_path) {
            Ok(c) => c,
            Err(_) => {
                self.status_message = "Cannot read version meta.".to_string();
                return;
            }
        };

        let meta: miao_core::version::meta::VersionMeta = match serde_json::from_str(&meta_content)
        {
            Ok(m) => m,
            Err(_) => {
                self.status_message = "Cannot parse version meta.".to_string();
                return;
            }
        };

        let required = meta.required_java_major();
        let java_installations = java::detect_system_java();
        if java::find_compatible_java(&java_installations, required).is_some() {
            self.status_message = format!("Java {} already available.", required);
            return;
        }

        self.installing = true;
        self.status_message = format!("Downloading Java {}...", required);

        let tx = self.tx.clone();
        let java_dir = self.config.data_dir.join("java");

        tokio::spawn(async move {
            let http = reqwest::Client::new();
            match miao_core::java::download::fetch_latest_asset(&http, required).await {
                Ok(asset) => {
                    let _ = tx.send(AsyncMessage::InstallProgress(format!(
                        "Downloading Java {} ({:.1} MB)...",
                        required,
                        asset.binary.package.size as f64 / 1_000_000.0
                    )));
                    match miao_core::java::download::download_and_extract_java(
                        &http, &asset, &java_dir,
                    )
                    .await
                    {
                        Ok(path) => {
                            let _ = tx.send(AsyncMessage::JavaDownloadDone(format!(
                                "✓ Java {} installed at {}",
                                required,
                                path.display()
                            )));
                        }
                        Err(e) => {
                            let _ = tx.send(AsyncMessage::JavaDownloadDone(format!(
                                "✗ Java download failed: {}",
                                e
                            )));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AsyncMessage::JavaDownloadDone(format!(
                        "✗ No Java {} available: {}",
                        required, e
                    )));
                }
            }
        });
    }

    pub fn start_create_instance(&mut self) {
        if self.versions.is_empty() {
            self.status_message = "Versions not loaded yet. Wait...".to_string();
            return;
        }
        self.create_name.clear();
        self.create_version_cursor = 0;
        self.create_loader_cursor = 0;
        self.loader_versions.clear();
        self.loading_loader_versions = false;
        self.input_mode = InputMode::CreateName;
        self.status_message = "New instance - Enter name (Enter=next, Esc=cancel):".to_string();
    }

    pub fn fetch_loader_versions_for_version(&mut self, mc_version: &str) {
        if self.loading_loader_versions {
            return;
        }
        self.loading_loader_versions = true;
        self.loader_versions.clear();
        self.status_message = format!("Loading loader versions for {}...", mc_version);

        let tx = self.tx.clone();
        let http = reqwest::Client::new();
        let mc_version = mc_version.to_string();

        tokio::spawn(async move {
            let versions =
                miao_core::modloader::fetch_all_loader_versions(&http, &mc_version).await;
            match versions {
                Ok(v) => {
                    let _ = tx.send(AsyncMessage::LoaderVersionsLoaded(mc_version, v));
                }
                Err(e) => {
                    let _ = tx.send(AsyncMessage::InstallError(format!(
                        "Failed to load loader versions: {}",
                        e
                    )));
                }
            }
        });
    }

    pub fn create_name_confirm(&mut self) {
        if self.create_name.is_empty()
            && let Some(ver) = self.versions.get(self.create_version_cursor)
        {
            self.create_name = ver.id.clone();
        }
        self.input_mode = InputMode::CreateSelectVersion;
        self.status_message = "Select MC version [j/k] navigate, [Enter] next:".to_string();
    }

    pub fn create_version_confirm(&mut self) {
        let mc_version = self
            .versions
            .get(self.create_version_cursor)
            .map(|v| v.id.clone());
        if let Some(mc_version) = mc_version {
            self.fetch_loader_versions_for_version(&mc_version);
        }
        self.input_mode = InputMode::CreateSelectLoader;
        self.create_loader_cursor = 0;
        self.loader_version_cursor = 0;
        self.status_message = "Select mod loader [j/k] navigate, [Enter] create:".to_string();
    }

    pub fn get_available_loaders(&self) -> Vec<(usize, &'static str, bool)> {
        let mut loaders = Vec::new();
        loaders.push((0, "None (Vanilla)", true));

        let loader_types = [
            (1, "Fabric", ModLoaderType::Fabric),
            (2, "Quilt", ModLoaderType::Quilt),
            (3, "NeoForge", ModLoaderType::NeoForge),
            (4, "Forge", ModLoaderType::Forge),
        ];

        for (idx, name, loader_type) in loader_types {
            let available = self.loader_versions.contains_key(&loader_type);
            loaders.push((idx, name, available));
        }

        loaders
    }

    pub fn get_current_loader_versions(&self) -> Vec<&ModLoaderVersion> {
        if self.create_loader_cursor == 0 {
            return Vec::new();
        }
        ModLoaderType::from_index(self.create_loader_cursor - 1)
            .and_then(|lt| self.loader_versions.get(&lt))
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    pub fn create_loader_confirm(&mut self) {
        let Some(ver) = self.versions.get(self.create_version_cursor).cloned() else {
            self.input_mode = InputMode::Normal;
            return;
        };

        let instance_name = if self.create_name.is_empty() {
            ver.id.clone()
        } else {
            self.create_name.clone()
        };

        let loader_type = if self.create_loader_cursor == 0 {
            None
        } else {
            let Some(lt) = ModLoaderType::from_index(self.create_loader_cursor - 1) else {
                self.input_mode = InputMode::Normal;
                return;
            };

            if !self.loader_versions.contains_key(&lt) {
                self.status_message = format!("{} is not available for {}", lt, ver.id);
                return;
            }

            let versions = self.loader_versions.get(&lt).unwrap();
            if let Some(selected) = versions.get(self.loader_version_cursor) {
                Some((lt.as_str().to_string(), selected.version.clone()))
            } else if let Some(first) = versions.first() {
                Some((lt.as_str().to_string(), first.version.clone()))
            } else {
                self.status_message = format!("No {} versions available", lt);
                return;
            }
        };

        self.input_mode = InputMode::Normal;

        if self.installing {
            self.status_message = "Already installing...".to_string();
            return;
        }

        self.installing = true;
        self.status_message = format!("Creating '{}'...", instance_name);

        let tx = self.tx.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = do_create_instance_with_version(
                &ver,
                &instance_name,
                loader_type
                    .as_ref()
                    .map(|(lt, lv)| (lt.as_str(), lv.as_str())),
                &config,
                &tx,
            )
            .await
            {
                let _ = tx.send(AsyncMessage::InstallError(e.to_string()));
            }
        });
    }

    pub fn create_version_next(&mut self) {
        if !self.versions.is_empty() {
            self.create_version_cursor = (self.create_version_cursor + 1) % self.versions.len();
        }
    }

    pub fn create_version_prev(&mut self) {
        if !self.versions.is_empty() {
            self.create_version_cursor = if self.create_version_cursor == 0 {
                self.versions.len() - 1
            } else {
                self.create_version_cursor - 1
            };
        }
    }

    pub fn create_loader_next(&mut self) {
        let available = self.get_available_loaders();
        let current_idx = available
            .iter()
            .position(|(idx, _, _)| *idx == self.create_loader_cursor)
            .unwrap_or(0);
        let next_idx = (current_idx + 1) % available.len();
        self.create_loader_cursor = available[next_idx].0;
        self.loader_version_cursor = 0;
    }

    pub fn create_loader_prev(&mut self) {
        let available = self.get_available_loaders();
        let current_idx = available
            .iter()
            .position(|(idx, _, _)| *idx == self.create_loader_cursor)
            .unwrap_or(0);
        let prev_idx = if current_idx == 0 {
            available.len() - 1
        } else {
            current_idx - 1
        };
        self.create_loader_cursor = available[prev_idx].0;
        self.loader_version_cursor = 0;
    }

    pub fn loader_version_next(&mut self) {
        let versions = self.get_current_loader_versions();
        if !versions.is_empty() {
            self.loader_version_cursor = (self.loader_version_cursor + 1) % versions.len();
        }
    }

    pub fn loader_version_prev(&mut self) {
        let versions = self.get_current_loader_versions();
        if !versions.is_empty() {
            self.loader_version_cursor = if self.loader_version_cursor == 0 {
                versions.len() - 1
            } else {
                self.loader_version_cursor - 1
            };
        }
    }

    pub fn loader_next(&mut self) {
        self.loader_cursor = (self.loader_cursor + 1) % LOADER_OPTIONS.len();
    }

    pub fn loader_prev(&mut self) {
        self.loader_cursor = if self.loader_cursor == 0 {
            LOADER_OPTIONS.len() - 1
        } else {
            self.loader_cursor - 1
        };
    }

    pub fn confirm_loader(&mut self) {
        let Some(inst) = self.instances.get(self.selected_index) else {
            self.input_mode = InputMode::Normal;
            return;
        };

        let loader_name = LOADER_OPTIONS[self.loader_cursor];
        self.input_mode = InputMode::Normal;

        if self.installing {
            self.status_message = "Already installing...".to_string();
            return;
        }

        self.installing = true;
        self.status_message = format!("Installing {} for '{}'...", loader_name, inst.name);

        let tx = self.tx.clone();
        let config = self.config.clone();
        let instance_name = inst.name.clone();
        let mc_version = inst.minecraft_version.clone();
        let loader_idx = self.loader_cursor;

        tokio::spawn(async move {
            let result =
                do_loader_install(&instance_name, &mc_version, loader_idx, &config, &tx).await;
            if let Err(e) = result {
                let _ = tx.send(AsyncMessage::InstallError(e.to_string()));
            }
        });
    }

    fn current_list_len(&self) -> usize {
        match self.active_tab() {
            Tab::Instances => self.instances.len(),
            Tab::Versions => self.versions.len(),
            Tab::Accounts => self.config.accounts.len(),
            _ => 0,
        }
    }
}

async fn do_loader_install(
    instance_name: &str,
    mc_version: &str,
    loader_idx: usize,
    config: &LauncherConfig,
    tx: &mpsc::UnboundedSender<AsyncMessage>,
) -> anyhow::Result<()> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

    let http = reqwest::Client::new();
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);

    let (loader_type, loader_ver) = match loader_idx {
        0 => {
            let _ = tx.send(AsyncMessage::InstallProgress(
                "Fetching Fabric versions...".to_string(),
            ));
            let versions = fabric::fetch_loader_versions(&http, mc_version).await?;
            let ver = versions
                .iter()
                .find(|v| v.loader.stable)
                .or(versions.first())
                .map(|v| v.loader.version.clone())
                .ok_or_else(|| anyhow::anyhow!("No Fabric versions for {}", mc_version))?;

            let _ = tx.send(AsyncMessage::InstallProgress(format!(
                "Installing Fabric {}...",
                ver
            )));
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
            let _ = tx.send(AsyncMessage::InstallProgress(
                "Fetching Quilt versions...".to_string(),
            ));
            let versions = quilt::fetch_loader_versions(&http, mc_version).await?;
            let ver = versions
                .first()
                .map(|v| v.loader.version.clone())
                .ok_or_else(|| anyhow::anyhow!("No Quilt versions for {}", mc_version))?;

            let _ = tx.send(AsyncMessage::InstallProgress(format!(
                "Installing Quilt {}...",
                ver
            )));
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
            let _ = tx.send(AsyncMessage::InstallProgress(
                "Fetching NeoForge versions...".to_string(),
            ));
            let versions = neoforge::fetch_versions(&http, mc_version).await?;
            let ver = versions
                .first()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No NeoForge versions for {}", mc_version))?;

            let _ = tx.send(AsyncMessage::InstallProgress(format!(
                "Installing NeoForge {}...",
                ver
            )));
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
            let _ = tx.send(AsyncMessage::InstallProgress(
                "Fetching Forge versions...".to_string(),
            ));
            let ver = forge::fetch_recommended_version(&http, mc_version)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No Forge versions for {}", mc_version))?;

            let _ = tx.send(AsyncMessage::InstallProgress(format!(
                "Installing Forge {}...",
                ver
            )));
            let profile = forge::fetch_install_profile(&http, mc_version, &ver).await?;
            let tasks = forge::collect_library_downloads(&profile, config);
            let dm = DownloadManager::new(
                config.download_mirror.clone(),
                config.max_concurrent_downloads,
            );
            dm.download_all(tasks).await?;
            (ModLoaderType::Forge, ver)
        }
        _ => anyhow::bail!("Invalid loader index"),
    };

    let mut inst = Instance::load_from(&instance_dir)?;
    inst.mod_loader = Some(miao_core::instance::ModLoaderConfig {
        loader_type: loader_type.clone(),
        version: loader_ver.clone(),
    });
    inst.save_to(&instance_dir)?;

    let _ = tx.send(AsyncMessage::InstallDone(format!(
        "{} {} for '{}'",
        loader_type, loader_ver, instance_name
    )));
    Ok(())
}

async fn do_create_instance_with_version(
    ver: &VersionInfo,
    instance_name: &str,
    loader: Option<(&str, &str)>,
    config: &LauncherConfig,
    tx: &mpsc::UnboundedSender<AsyncMessage>,
) -> anyhow::Result<()> {
    let http = reqwest::Client::new();

    let _ = tx.send(AsyncMessage::InstallProgress(format!(
        "Fetching metadata for {}...",
        ver.id
    )));

    let meta =
        miao_core::version::install::fetch_version_meta(&http, &ver.url, &config.download_mirror)
            .await?;

    miao_core::version::install::save_version_meta(&meta, config)?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, config, &config.download_mirror);

    let _ = tx.send(AsyncMessage::InstallProgress(format!(
        "Downloading {} libraries...",
        tasks.len()
    )));

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

        let _ = tx.send(AsyncMessage::InstallProgress(format!(
            "Downloading {} assets...",
            asset_tasks.len()
        )));

        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks).await?;
    }

    let mut inst = Instance::new(instance_name, &ver.id);

    if let Some((loader_type_str, loader_version)) = loader {
        let _ = tx.send(AsyncMessage::InstallProgress(format!(
            "Installing {} {}...",
            loader_type_str, loader_version
        )));

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

    let _ = tx.send(AsyncMessage::InstallDone(instance_name.to_string()));
    Ok(())
}
