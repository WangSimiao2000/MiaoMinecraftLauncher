use anyhow::Result;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};

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

pub struct App {
    pub config: LauncherConfig,
    pub current_tab: usize,
    pub selected_index: usize,
    pub instances: Vec<Instance>,
    pub status_message: String,
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
            status_message: "Ready. Press 'q' to quit, Tab to switch panels.".to_string(),
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
                    self.status_message =
                        format!("Selected: {} (MC {})", inst.name, inst.minecraft_version);
                }
            }
            _ => {
                self.status_message = "Action not yet implemented.".to_string();
            }
        }
    }

    pub fn go_back(&mut self) {
        self.status_message = "Ready.".to_string();
    }

    fn current_list_len(&self) -> usize {
        match self.active_tab() {
            Tab::Instances => self.instances.len(),
            Tab::Accounts => self.config.accounts.len(),
            _ => 0,
        }
    }
}
