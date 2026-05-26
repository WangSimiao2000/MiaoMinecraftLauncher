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
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    Mods,
    Resources,
    Worlds,
    Log,
    Settings,
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
}

#[derive(Debug, Clone, Default)]
pub struct LoaderUiState {
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
pub struct GameLogState {
    pub lines: VecDeque<String>,
    pub running: bool,
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
}

impl NewInstanceInput {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ─── i18n ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::Chinese];

    pub fn name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }
}

pub struct I18n;

impl I18n {
    pub fn t(lang: Language, key: &'static str) -> &'static str {
        match (lang, key) {
            (Language::English, "ready") => "Ready",
            (Language::Chinese, "ready") => "就绪",
            (Language::English, "settings") => "Settings",
            (Language::Chinese, "settings") => "设置",
            (Language::English, "back") => "< Back",
            (Language::Chinese, "back") => "< 返回",
            (Language::English, "instances") => "Instances",
            (Language::Chinese, "instances") => "实例",
            (Language::English, "new") => "+ New",
            (Language::Chinese, "new") => "+ 新建",
            (Language::English, "import") => "Import",
            (Language::Chinese, "import") => "导入",
            (Language::English, "launch") => "▶ Launch",
            (Language::Chinese, "launch") => "▶ 启动",
            (Language::English, "export") => "Export",
            (Language::Chinese, "export") => "导出",
            (Language::English, "open") => "Open",
            (Language::Chinese, "open") => "打开",
            (Language::English, "delete") => "Delete",
            (Language::Chinese, "delete") => "删除",
            (Language::English, "cancel") => "Cancel",
            (Language::Chinese, "cancel") => "取消",
            (Language::English, "confirm") => "Confirm?",
            (Language::Chinese, "confirm") => "确认？",
            (Language::English, "yes") => "Yes",
            (Language::Chinese, "yes") => "是",
            (Language::English, "no") => "No",
            (Language::Chinese, "no") => "否",
            (Language::English, "save") => "Save",
            (Language::Chinese, "save") => "保存",
            (Language::English, "apply") => "Apply",
            (Language::Chinese, "apply") => "应用",
            (Language::English, "tab_mods") => "Mods",
            (Language::Chinese, "tab_mods") => "模组",
            (Language::English, "tab_resources") => "Resources",
            (Language::Chinese, "tab_resources") => "资源包",
            (Language::English, "tab_worlds") => "Worlds",
            (Language::Chinese, "tab_worlds") => "世界",
            (Language::English, "tab_log") => "Log",
            (Language::Chinese, "tab_log") => "日志",
            (Language::English, "tab_settings") => "Settings",
            (Language::Chinese, "tab_settings") => "设置",
            (Language::English, "account") => "Account",
            (Language::Chinese, "account") => "账号",
            (Language::English, "appearance") => "Appearance",
            (Language::Chinese, "appearance") => "外观",
            (Language::English, "theme") => "Theme",
            (Language::Chinese, "theme") => "主题",
            (Language::English, "data") => "Data",
            (Language::Chinese, "data") => "数据",
            (Language::English, "java") => "Java",
            (Language::Chinese, "java") => "Java",
            (Language::English, "about") => "About",
            (Language::Chinese, "about") => "关于",
            (Language::English, "language") => "Language",
            (Language::Chinese, "language") => "语言",
            (Language::English, "memory") => "Memory",
            (Language::Chinese, "memory") => "内存",
            (Language::English, "resolution") => "Resolution",
            (Language::Chinese, "resolution") => "分辨率",
            (Language::English, "jvm_args") => "JVM Arguments",
            (Language::Chinese, "jvm_args") => "JVM 参数",
            (Language::English, "java_path") => "Java Path",
            (Language::Chinese, "java_path") => "Java 路径",
            (Language::English, "instance_settings") => "Instance Settings",
            (Language::Chinese, "instance_settings") => "实例设置",
            (Language::English, "memory_min") => "Min (MB)",
            (Language::Chinese, "memory_min") => "最小 (MB)",
            (Language::English, "memory_max") => "Max (MB)",
            (Language::Chinese, "memory_max") => "最大 (MB)",
            (Language::English, "width") => "Width",
            (Language::Chinese, "width") => "宽度",
            (Language::English, "height") => "Height",
            (Language::Chinese, "height") => "高度",
            (Language::English, "saved") => "Settings saved.",
            (Language::Chinese, "saved") => "设置已保存。",
            (Language::English, "mirror") => "Mirror",
            (Language::Chinese, "mirror") => "下载源",
            (Language::English, "mirror_official") => "Official",
            (Language::Chinese, "mirror_official") => "官方",
            (Language::English, "mirror_bmclapi") => "BMCLAPI",
            (Language::Chinese, "mirror_bmclapi") => "BMCLAPI",
            (Language::English, "mirror_custom") => "Custom",
            (Language::Chinese, "mirror_custom") => "自定义",
            (Language::English, "max_concurrent") => "Max concurrent downloads",
            (Language::Chinese, "max_concurrent") => "最大并发下载数",
            (Language::English, "show_snapshots") => "Snapshots",
            (Language::Chinese, "show_snapshots") => "快照版",
            (Language::English, "show_old_beta") => "Old Beta",
            (Language::Chinese, "show_old_beta") => "旧Beta版",
            (Language::English, "show_old_alpha") => "Old Alpha",
            (Language::Chinese, "show_old_alpha") => "旧Alpha版",
            (Language::English, "create_instance") => "Create New Instance",
            (Language::Chinese, "create_instance") => "创建新实例",
            (Language::English, "game_log") => "Game Log",
            (Language::Chinese, "game_log") => "游戏日志",
            (Language::English, "clear_log") => "Clear",
            (Language::Chinese, "clear_log") => "清除",
            (Language::English, "no_log") => "No log yet. Launch the game first.",
            (Language::Chinese, "no_log") => "暂无日志，请先启动游戏。",
            (Language::English, "welcome") => "Welcome to MMCL",
            (Language::Chinese, "welcome") => "欢迎使用 MMCL",
            (Language::English, "welcome_hint") => {
                "Select an instance from the left panel,\nor click '+ New' to create one."
            }
            (Language::Chinese, "welcome_hint") => {
                "从左侧面板选择一个实例，\n或点击 '+ 新建' 创建一个。"
            }
            (Language::English, "no_instances") => "No instances yet",
            (Language::Chinese, "no_instances") => "暂无实例",
            (Language::English, "no_instances_hint") => "Click '+ New' to create one",
            (Language::Chinese, "no_instances_hint") => "点击 '+ 新建' 创建一个",
            (Language::English, "active_account") => "Active Account",
            (Language::Chinese, "active_account") => "当前账号",
            (Language::English, "add_account") => "Add Account",
            (Language::Chinese, "add_account") => "添加账号",
            (Language::English, "offline_account") => "Offline Account",
            (Language::Chinese, "offline_account") => "离线账号",
            (Language::English, "ms_account") => "Microsoft Account",
            (Language::Chinese, "ms_account") => "微软账号",
            (Language::English, "no_accounts") => "No accounts configured yet.",
            (Language::Chinese, "no_accounts") => "暂未配置任何账号。",
            (Language::English, "sign_in") => "Sign In",
            (Language::Chinese, "sign_in") => "登录",
            (Language::English, "update_available") => "Update available",
            (Language::Chinese, "update_available") => "有新版本可用",
            _ => key,
        }
    }
}
