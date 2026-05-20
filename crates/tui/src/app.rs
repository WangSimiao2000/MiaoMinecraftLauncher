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
}

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
    pub ms_device_code: Option<DeviceCodeResponse>,
    pub rx: mpsc::UnboundedReceiver<AsyncMessage>,
    pub tx: mpsc::UnboundedSender<AsyncMessage>,
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
            status_message: "[q]uit [Tab]switch [j/k]nav [i]nstall [l]aunch [a]ccount [m]icrosoft"
                .to_string(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            loading: true,
            installing: false,
            ms_device_code: None,
            rx,
            tx,
        })
    }

    pub fn process_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                AsyncMessage::VersionsLoaded(v) => {
                    self.versions = v;
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

    pub fn install_selected(&mut self) {
        if self.installing {
            self.status_message = "Already installing...".to_string();
            return;
        }

        let Some(ver) = self.versions.get(self.selected_index).cloned() else {
            self.status_message = "No version selected.".to_string();
            return;
        };

        self.installing = true;
        self.status_message = format!("Installing MC {}...", ver.id);

        let tx = self.tx.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            if let Err(e) = do_install(&ver, &config, &tx).await {
                let _ = tx.send(AsyncMessage::InstallError(e.to_string()));
            }
        });
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

    fn current_list_len(&self) -> usize {
        match self.active_tab() {
            Tab::Instances => self.instances.len(),
            Tab::Versions => self.versions.len(),
            Tab::Accounts => self.config.accounts.len(),
            _ => 0,
        }
    }
}

async fn do_install(
    ver: &VersionInfo,
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
    let task_count = tasks.len();

    let _ = tx.send(AsyncMessage::InstallProgress(format!(
        "Downloading {} libraries...",
        task_count
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
        let asset_count = asset_tasks.len();

        let _ = tx.send(AsyncMessage::InstallProgress(format!(
            "Downloading {} assets...",
            asset_count
        )));

        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        );
        dm2.download_all(asset_tasks).await?;
    }

    let instance_name = &ver.id;
    let inst = Instance::new(instance_name, &ver.id);
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    let _ = tx.send(AsyncMessage::InstallDone(instance_name.to_string()));
    Ok(())
}
