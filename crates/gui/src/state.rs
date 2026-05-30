use std::collections::HashMap;
use std::collections::VecDeque;

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
    Appearance,
    Data,
    Java,
    Help,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Mods,
    Resources,
    Worlds,
    Log,
    ModpackSync,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModSource {
    #[default]
    Modrinth,
    CurseForge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    None,
    NewInstance,
    ConfirmJavaDownload {
        instance_idx: usize,
        java_major: u32,
    },
}

#[derive(Debug, Clone, Default)]
pub struct VersionsUiState {
    pub all_versions: Vec<VersionInfo>,
    pub versions: Vec<VersionInfo>,
    pub loading: bool,
    pub show_snapshots: bool,
    pub show_old_beta: bool,
    pub show_old_alpha: bool,
}

#[derive(Debug, Clone, Default)]
pub struct InstallProgress {
    pub total: usize,
    pub completed: usize,
    pub label: String,
}

#[derive(Debug, Clone, Default)]
pub struct AuthUiState {
    pub device_code: Option<DeviceCodeResponse>,
    pub logging_in: bool,
    pub authlib_server_url: String,
    pub authlib_email: String,
    pub authlib_password: String,
    pub authlib_logging_in: bool,
}

#[derive(Debug, Clone, Default)]
pub struct LoaderUiState {
    pub versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    pub loading: bool,
    pub failed: Vec<ModLoaderType>,
}

#[derive(Debug, Clone, Default)]
pub struct ModSearchState {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub versions: Vec<ProjectVersion>,
    pub selected: Option<usize>,
    pub searching: bool,
    pub active: bool,
    pub offset: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Default)]
pub struct CfSearchState {
    pub query: String,
    pub results: Vec<miao_core::curseforge::api::CfMod>,
    pub files: Vec<miao_core::curseforge::api::CfFile>,
    pub selected_mod_id: Option<u32>,
    pub selected_mod_name: Option<String>,
    pub searching: bool,
    pub active: bool,
    pub offset: u32,
    pub total_count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct CfPendingInstall {
    pub mod_name: String,
    pub deps: Vec<miao_core::curseforge::api::CfResolvedDep>,
    pub instance_dir: std::path::PathBuf,
    pub mc_version: String,
    pub loader: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct PendingModInstall {
    pub project_slug: String,
    pub deps: Vec<ResolvedDep>,
    pub instance_dir: std::path::PathBuf,
    pub mc_version: String,
    pub loader: String,
}

/// Unified pending install enum that wraps both Modrinth and CurseForge pending installs.
#[derive(Debug, Clone)]
pub enum PendingInstall {
    Modrinth(PendingModInstall),
    CurseForge(CfPendingInstall),
}

#[derive(Debug, Clone, Default)]
pub struct GameLogState {
    pub lines: VecDeque<String>,
    pub running: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ModpackSourceState {
    pub current_source_id: Option<String>,
    pub manifest: Option<miao_core::modpack_source::Manifest>,
    pub manifest_loading: bool,
    pub manifest_error: Option<String>,
    pub selected_pack_idx: Option<usize>,
    pub selected_mc_version: Option<String>,
    pub current_subscription_report: Option<miao_core::modpack_source::ResolutionReport>,
    pub sources: std::collections::HashMap<String, miao_core::modpack_source::ModpackSource>,
    pub resolving: Option<String>,
    pub pending_confirm: Option<PendingModpackInstall>,
}

#[derive(Debug, Clone)]
pub struct PendingModpackInstall {
    pub source_id: String,
    pub pack_id: String,
    pub pack_url: String,
    pub instance_name: String,
    pub config: Box<miao_core::config::LauncherConfig>,
    pub report: Box<miao_core::modpack_source::ResolutionReport>,
    pub pack_raw: Vec<u8>,
    pub pack: Box<miao_core::modpack_source::Pack>,
}

impl GameLogState {
    pub const MAX_LINES: usize = 2000;
}

#[derive(Debug, Clone, Default)]
pub struct InstanceSettingsEdit {
    pub memory_max: String,
    pub memory_min: String,
    pub resolution_width: String,
    pub resolution_height: String,
    pub java_path: String,
    pub jvm_args: String,
    pub dirty: bool,
}

#[derive(Default)]
pub struct NewInstanceInput {
    pub name: String,
    pub version_idx: usize,
    pub loader: usize,
    pub loader_version_idx: usize,
    pub mode: NewInstanceMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NewInstanceMode {
    #[default]
    Custom,
    Modpack,
}

impl NewInstanceInput {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ─── i18n ────────────────────────────────────────────────────────────────────

use std::path::Path;
use std::sync::{LazyLock, RwLock};

type LocaleMap = HashMap<&'static str, &'static str>;

#[derive(Debug, Clone)]
pub struct LocaleEntry {
    pub id: String,
    pub name: String,
}

struct LocaleRegistry {
    locales: HashMap<String, LocaleMap>,
    order: Vec<LocaleEntry>,
}

impl LocaleRegistry {
    fn new() -> Self {
        let mut reg = Self {
            locales: HashMap::new(),
            order: Vec::new(),
        };
        reg.register("en", include_str!("../locales/en.json"));
        reg.register("zh", include_str!("../locales/zh.json"));
        reg.register("ja", include_str!("../locales/ja.json"));
        reg
    }

    fn register(&mut self, id: &str, json: &str) {
        let raw: HashMap<String, String> = match serde_json::from_str(json) {
            Ok(m) => m,
            Err(_) => return,
        };
        let name = raw.get("_name").cloned().unwrap_or_else(|| id.to_string());
        let map: LocaleMap = raw
            .into_iter()
            .filter(|(k, _)| !k.starts_with('_'))
            .map(|(k, v)| {
                let k: &'static str = Box::leak(k.into_boxed_str());
                let v: &'static str = Box::leak(v.into_boxed_str());
                (k, v)
            })
            .collect();
        if !self.locales.contains_key(id) {
            self.order.push(LocaleEntry {
                id: id.to_string(),
                name,
            });
        }
        self.locales.insert(id.to_string(), map);
    }

    fn get(&self, lang: &str, key: &'static str) -> &'static str {
        if let Some(map) = self.locales.get(lang)
            && let Some(val) = map.get(key)
        {
            return val;
        }
        if let Some(map) = self.locales.get("en")
            && let Some(val) = map.get(key)
        {
            return val;
        }
        key
    }

    fn available(&self) -> Vec<LocaleEntry> {
        self.order.clone()
    }
}

static REGISTRY: LazyLock<RwLock<LocaleRegistry>> =
    LazyLock::new(|| RwLock::new(LocaleRegistry::new()));

pub struct I18n;

impl I18n {
    pub fn t(lang: &str, key: &'static str) -> &'static str {
        let reg = REGISTRY.read().unwrap();
        reg.get(lang, key)
    }

    pub fn available_languages() -> Vec<LocaleEntry> {
        let reg = REGISTRY.read().unwrap();
        reg.available()
    }

    pub fn load_external_locales(locales_dir: &Path) {
        let Ok(entries) = std::fs::read_dir(locales_dir) else {
            return;
        };
        let mut reg = REGISTRY.write().unwrap();

        let builtin_ids: Vec<String> = ["en", "zh", "ja"].iter().map(|s| s.to_string()).collect();
        reg.locales.retain(|id, _| builtin_ids.contains(id));
        reg.order.retain(|e| builtin_ids.contains(&e.id));

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                let id = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if id.starts_with('_') {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let leaked: &'static str = Box::leak(content.into_boxed_str());
                    reg.register(&id, leaked);
                }
            }
        }
    }
}
