use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use miao_core::auth::microsoft::DeviceCodeResponse;
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::modrinth::api::{ProjectVersion, ResolvedDep, SearchHit};
use miao_core::version::VersionInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Main,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Account,
    Data,
    Java,
    About,
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
pub struct VersionsState {
    pub versions: Vec<VersionInfo>,
    pub loading: bool,
}

#[derive(Debug, Clone, Default)]
pub struct InstallState {
    pub status: Option<String>,
    pub installing: bool,
    pub progress_total: usize,
    pub progress_completed: usize,
    pub progress_label: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuthState {
    pub device_code: Option<DeviceCodeResponse>,
    pub logging_in: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LoaderState {
    pub versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    pub loading: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ModSearchState {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub versions: Vec<ProjectVersion>,
    pub selected: Option<usize>,
    pub searching: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PendingModInstall {
    pub project_slug: String,
    pub deps: Vec<ResolvedDep>,
    pub instance_dir: std::path::PathBuf,
    pub mc_version: String,
    pub loader: String,
}

#[derive(Debug, Clone, Default)]
pub struct AsyncState {
    pub versions: VersionsState,
    pub install: InstallState,
    pub auth: AuthState,
    pub loader: LoaderState,
    pub mod_search_hits: Option<Vec<SearchHit>>,
    pub mod_versions: Option<Vec<ProjectVersion>>,
    pub pending_mod_install: Option<PendingModInstall>,
}

pub type SharedAsyncState = Arc<Mutex<AsyncState>>;

#[derive(Default)]
pub struct NewInstanceInput {
    pub name: String,
    pub version_idx: usize,
    pub loader: usize,
    pub loader_version_idx: usize,
}

impl NewInstanceInput {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
