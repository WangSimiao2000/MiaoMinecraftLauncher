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
pub struct CfSearchState {
    pub query: String,
    pub results: Vec<miao_core::curseforge::api::CfMod>,
    pub files: Vec<miao_core::curseforge::api::CfFile>,
    pub selected_mod_id: Option<u32>,
    pub selected_mod_name: Option<String>,
    pub searching: bool,
    pub active: bool,
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
            (Language::English, "back") => "← Back",
            (Language::Chinese, "back") => "← 返回",
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

            (Language::English, "search_mods") => "🔍 Search Mods",
            (Language::Chinese, "search_mods") => "🔍 搜索模组",
            (Language::English, "close_search") => "⊘ Close Search",
            (Language::Chinese, "close_search") => "⊘ 关闭搜索",
            (Language::English, "open_folder") => "Open folder",
            (Language::Chinese, "open_folder") => "打开文件夹",
            (Language::English, "search") => "Search:",
            (Language::Chinese, "search") => "搜索：",
            (Language::English, "go") => "Go",
            (Language::Chinese, "go") => "搜索",
            (Language::English, "install") => "Install",
            (Language::Chinese, "install") => "安装",
            (Language::English, "install_all") => "Install All",
            (Language::Chinese, "install_all") => "全部安装",
            (Language::English, "only_this_mod") => "Only This Mod",
            (Language::Chinese, "only_this_mod") => "仅安装此模组",
            (Language::English, "confirm_install") => "Confirm Installation",
            (Language::Chinese, "confirm_install") => "确认安装",
            (Language::English, "confirm_install_cf") => "Confirm Installation (CurseForge)",
            (Language::Chinese, "confirm_install_cf") => "确认安装 (CurseForge)",
            (Language::English, "required_deps") => "Required dependencies",
            (Language::Chinese, "required_deps") => "必需依赖",
            (Language::English, "versions_for") => "Versions for",
            (Language::Chinese, "versions_for") => "版本列表：",
            (Language::English, "no_mods") => "No mods installed",
            (Language::Chinese, "no_mods") => "未安装模组",
            (Language::English, "no_mods_hint") => "Click \"Search Mods\" to find and install mods",
            (Language::Chinese, "no_mods_hint") => "点击\"搜索模组\"查找并安装模组",
            (Language::English, "no_resource_packs") => "No resource packs",
            (Language::Chinese, "no_resource_packs") => "暂无资源包",
            (Language::English, "no_resource_packs_hint") => {
                "Drop .zip packs into the resourcepacks folder"
            }
            (Language::Chinese, "no_resource_packs_hint") => {
                "将 .zip 资源包放入 resourcepacks 文件夹"
            }
            (Language::English, "shaders") => "Shaders",
            (Language::Chinese, "shaders") => "光影",
            (Language::English, "no_shaders") => "No shaders",
            (Language::Chinese, "no_shaders") => "暂无光影",
            (Language::English, "no_shaders_hint") => {
                "Drop shader packs into the shaderpacks folder"
            }
            (Language::Chinese, "no_shaders_hint") => "将光影包放入 shaderpacks 文件夹",
            (Language::English, "no_worlds") => "No worlds yet.",
            (Language::Chinese, "no_worlds") => "暂无存档。",
            (Language::English, "no_worlds_hint") => "Launch the game to create one.",
            (Language::Chinese, "no_worlds_hint") => "启动游戏即可生成存档。",
            (Language::English, "today") => "today",
            (Language::Chinese, "today") => "今天",
            (Language::English, "yesterday") => "yesterday",
            (Language::Chinese, "yesterday") => "昨天",
            (Language::English, "days_ago") => "d ago",
            (Language::Chinese, "days_ago") => "天前",
            (Language::English, "del") => "Del",
            (Language::Chinese, "del") => "删除",
            (Language::English, "vanilla") => "Vanilla",
            (Language::Chinese, "vanilla") => "原版",
            (Language::English, "create") => "Create",
            (Language::Chinese, "create") => "创建",
            (Language::English, "add") => "Add",
            (Language::Chinese, "add") => "添加",
            (Language::English, "browse") => "Browse",
            (Language::Chinese, "browse") => "浏览",
            (Language::English, "clear") => "Clear",
            (Language::Chinese, "clear") => "清除",
            (Language::English, "refresh") => "Refresh",
            (Language::Chinese, "refresh") => "刷新",
            (Language::English, "download") => "Download",
            (Language::Chinese, "download") => "下载",
            (Language::English, "background") => "Background",
            (Language::Chinese, "background") => "背景图",
            (Language::English, "author") => "Author",
            (Language::Chinese, "author") => "作者",
            (Language::English, "username") => "Username",
            (Language::Chinese, "username") => "用户名",
            (Language::English, "cf_no_key") => {
                "CurseForge API key not configured. Set it in Settings > Data."
            }
            (Language::Chinese, "cf_no_key") => {
                "CurseForge API 密钥未配置，请在设置 > 数据中填写。"
            }
            (Language::English, "theme_desc") => "Choose a color theme for the launcher.",
            (Language::Chinese, "theme_desc") => "选择启动器的配色主题。",
            (Language::English, "data_dir") => "Data Directory",
            (Language::Chinese, "data_dir") => "数据目录",
            (Language::English, "data_dir_desc") => {
                "Where game files are stored (instances, libraries, assets)."
            }
            (Language::Chinese, "data_dir_desc") => "游戏文件存储位置（实例、库文件、资源）。",
            (Language::English, "data_dir_updated") => "Data directory updated.",
            (Language::Chinese, "data_dir_updated") => "数据目录已更新。",
            (Language::English, "apply_migrate") => "Apply & Migrate",
            (Language::Chinese, "apply_migrate") => "应用并迁移",
            (Language::English, "data_dir_migrated") => "Data directory migrated.",
            (Language::Chinese, "data_dir_migrated") => "数据目录已迁移。",
            (Language::English, "data_dir_partial") => {
                "Data directory set (some files could not be moved)."
            }
            (Language::Chinese, "data_dir_partial") => "数据目录已设置（部分文件无法移动）。",
            (Language::English, "cf_api_key") => "CurseForge API Key",
            (Language::Chinese, "cf_api_key") => "CurseForge API 密钥",
            (Language::English, "cf_api_key_desc") => {
                "Optional override. A built-in key is used by default."
            }
            (Language::Chinese, "cf_api_key_desc") => "可选覆盖，默认使用内置密钥。",
            (Language::English, "mirror_updated") => "Mirror updated.",
            (Language::Chinese, "mirror_updated") => "下载源已更新。",
            (Language::English, "java_installs") => "Java Installations",
            (Language::Chinese, "java_installs") => "Java 安装",
            (Language::English, "ms_login_hint") => "Open the link below and enter the code:",
            (Language::Chinese, "ms_login_hint") => "打开下方链接并输入验证码：",
            (Language::English, "ms_code") => "Code:",
            (Language::Chinese, "ms_code") => "验证码：",
            (Language::English, "ms_initializing") => "Initializing...",
            (Language::Chinese, "ms_initializing") => "正在初始化...",
            (Language::English, "ms_signin_desc") => "Sign in with your Microsoft account.",
            (Language::Chinese, "ms_signin_desc") => "使用微软账号登录。",
            (Language::English, "set_active") => "Set Active",
            (Language::Chinese, "set_active") => "设为活跃",
            (Language::English, "account_removed") => "Account removed.",
            (Language::Chinese, "account_removed") => "账号已移除。",
            (Language::English, "account_added") => "Account added.",
            (Language::Chinese, "account_added") => "账号已添加。",
            (Language::English, "skin_model") => "Model:",
            (Language::Chinese, "skin_model") => "模型：",
            (Language::English, "no_account_to_launch") => {
                "Please add an account before launching."
            }
            (Language::Chinese, "no_account_to_launch") => "请先添加账号再启动游戏。",
            _ => key,
        }
    }
}
