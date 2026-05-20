use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{DeviceCodeResponse, MicrosoftAuth, PollResult};
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::{self, Instance};
use miao_core::java;
use miao_core::launch::{LaunchOptions, build_launch_command};
use miao_core::version::VersionInfo;
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
}

pub const LOADER_OPTIONS: [&str; 5] = ["None (Vanilla)", "Fabric", "Quilt", "NeoForge", "Forge"];

#[derive(Debug, Clone)]
pub enum AsyncMessage {
    VersionsLoaded(Vec<VersionInfo>),
    InstallProgress(String),
    InstallDone(String),
    InstallError(String),
    MsDeviceCode(DeviceCodeResponse),
    MsLoginSuccess(String),
    MsLoginError(String),
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
            self.status_message = format!("No Java {} found!", required_java);
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

    pub fn start_create_instance(&mut self) {
        if self.versions.is_empty() {
            self.status_message = "Versions not loaded yet. Wait...".to_string();
            return;
        }
        self.create_name.clear();
        self.create_version_cursor = 0;
        self.create_loader_cursor = 0;
        self.input_mode = InputMode::CreateName;
        self.status_message = "New instance - Enter name (Enter=next, Esc=cancel):".to_string();
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
        self.input_mode = InputMode::CreateSelectLoader;
        self.create_loader_cursor = 0;
        self.status_message = "Select mod loader [j/k] navigate, [Enter] create:".to_string();
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
            Some(match self.create_loader_cursor {
                1 => "fabric",
                2 => "quilt",
                3 => "neoforge",
                4 => "forge",
                _ => unreachable!(),
            })
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
        let loader_type_owned = loader_type.map(|s| s.to_string());

        tokio::spawn(async move {
            if let Err(e) = do_create_instance(
                &ver,
                &instance_name,
                loader_type_owned.as_deref(),
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
        self.create_loader_cursor = (self.create_loader_cursor + 1) % LOADER_OPTIONS.len();
    }

    pub fn create_loader_prev(&mut self) {
        self.create_loader_cursor = if self.create_loader_cursor == 0 {
            LOADER_OPTIONS.len() - 1
        } else {
            self.create_loader_cursor - 1
        };
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

async fn do_create_instance(
    ver: &VersionInfo,
    instance_name: &str,
    loader_type: Option<&str>,
    config: &LauncherConfig,
    tx: &mpsc::UnboundedSender<AsyncMessage>,
) -> anyhow::Result<()> {
    use miao_core::modloader::{ModLoaderType, fabric, forge, neoforge, quilt};

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

    if let Some(lt) = loader_type {
        let loader_config = match lt {
            "fabric" => {
                let _ = tx.send(AsyncMessage::InstallProgress(
                    "Installing Fabric...".to_string(),
                ));
                let versions = fabric::fetch_loader_versions(&http, &ver.id).await?;
                let lv = versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Fabric versions"))?;
                let profile = fabric::fetch_profile(&http, &ver.id, &lv).await?;
                let tasks = fabric::collect_fabric_library_downloads(&profile, config);
                let dm = DownloadManager::new(
                    config.download_mirror.clone(),
                    config.max_concurrent_downloads,
                );
                dm.download_all(tasks).await?;
                miao_core::instance::ModLoaderConfig {
                    loader_type: ModLoaderType::Fabric,
                    version: lv,
                }
            }
            "quilt" => {
                let _ = tx.send(AsyncMessage::InstallProgress(
                    "Installing Quilt...".to_string(),
                ));
                let versions = quilt::fetch_loader_versions(&http, &ver.id).await?;
                let lv = versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Quilt versions"))?;
                let profile = quilt::fetch_profile(&http, &ver.id, &lv).await?;
                let tasks = quilt::collect_library_downloads(&profile, config);
                let dm = DownloadManager::new(
                    config.download_mirror.clone(),
                    config.max_concurrent_downloads,
                );
                dm.download_all(tasks).await?;
                miao_core::instance::ModLoaderConfig {
                    loader_type: ModLoaderType::Quilt,
                    version: lv,
                }
            }
            "neoforge" => {
                let _ = tx.send(AsyncMessage::InstallProgress(
                    "Installing NeoForge...".to_string(),
                ));
                let versions = neoforge::fetch_versions(&http, &ver.id).await?;
                let lv = versions
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No NeoForge versions"))?;
                let profile = neoforge::fetch_profile(&http, &lv).await?;
                let tasks = neoforge::collect_library_downloads(&profile, config);
                let dm = DownloadManager::new(
                    config.download_mirror.clone(),
                    config.max_concurrent_downloads,
                );
                dm.download_all(tasks).await?;
                miao_core::instance::ModLoaderConfig {
                    loader_type: ModLoaderType::NeoForge,
                    version: lv,
                }
            }
            "forge" => {
                let _ = tx.send(AsyncMessage::InstallProgress(
                    "Installing Forge...".to_string(),
                ));
                let lv = forge::fetch_recommended_version(&http, &ver.id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("No Forge versions"))?;
                let profile = forge::fetch_install_profile(&http, &ver.id, &lv).await?;
                let tasks = forge::collect_library_downloads(&profile, config);
                let dm = DownloadManager::new(
                    config.download_mirror.clone(),
                    config.max_concurrent_downloads,
                );
                dm.download_all(tasks).await?;
                miao_core::instance::ModLoaderConfig {
                    loader_type: ModLoaderType::Forge,
                    version: lv,
                }
            }
            _ => anyhow::bail!("Unknown loader"),
        };
        inst.mod_loader = Some(loader_config);
    }

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    let _ = tx.send(AsyncMessage::InstallDone(instance_name.to_string()));
    Ok(())
}
