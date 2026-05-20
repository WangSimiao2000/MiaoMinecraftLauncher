use anyhow::Result;
use miao_core::auth::AuthMethod;
use miao_core::auth::offline::create_offline_account;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};
use miao_core::version::VersionInfo;

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
}

impl App {
    pub fn new() -> Result<Self> {
        let config = LauncherConfig::load().unwrap_or_default();
        let instances = instance::list_instances(&config.instances_dir()).unwrap_or_default();

        Ok(Self {
            config,
            current_tab: 0,
            selected_index: 0,
            instances,
            versions: Vec::new(),
            status_message: "[q]uit [Tab]switch [j/k]nav [Enter]select [a]ccount [r]efresh"
                .to_string(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            loading: false,
        })
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
                    self.status_message = format!(
                        "Launch '{}' (MC {})? Press 'l' to launch.",
                        inst.name, inst.minecraft_version
                    );
                }
            }
            Tab::Versions => {
                if let Some(ver) = self.versions.get(self.selected_index) {
                    self.status_message = format!("Install MC {}? Press 'i' to install.", ver.id);
                }
            }
            Tab::Accounts => {
                self.status_message = "Press 'a' to add offline account.".to_string();
            }
            _ => {
                self.status_message = "Not yet implemented.".to_string();
            }
        }
    }

    pub fn go_back(&mut self) {
        if self.input_mode == InputMode::Input {
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
            self.status_message = "Cancelled.".to_string();
        }
    }

    pub fn add_offline_account(&mut self) {
        self.input_mode = InputMode::Input;
        self.input_buffer.clear();
        self.status_message = "Enter username (Enter to confirm, Esc to cancel):".to_string();
    }

    pub fn confirm_input(&mut self) {
        if self.input_mode == InputMode::Input && !self.input_buffer.is_empty() {
            let username = self.input_buffer.clone();
            let account = create_offline_account(&username);
            self.status_message = format!("Added account: {} ({})", account.username, account.uuid);
            self.config.accounts.push(AuthMethod::Offline(account));
            if self.config.active_account_index.is_none() {
                self.config.active_account_index = Some(0);
            }
            let _ = self.config.save();
            self.input_mode = InputMode::Normal;
            self.input_buffer.clear();
        }
    }

    pub fn set_versions(&mut self, versions: Vec<VersionInfo>) {
        self.versions = versions;
        self.loading = false;
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
