//! Message types for UI ↔ Controller communication.
//!
//! `AppCommand` flows from UI to the async controller.
//! `AppEvent` flows from the controller back to the UI.

use std::collections::HashMap;
use std::path::PathBuf;

use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::DeviceCodeResponse;
use miao_core::config::{DownloadMirror, LauncherConfig};
use miao_core::instance::Instance;
use miao_core::modloader::{ModLoaderType, ModLoaderVersion};
use miao_core::modrinth::api::{ProjectVersion, SearchHit};
use miao_core::version::VersionInfo;

use crate::state::PendingModInstall;

// ─── Commands (UI → Controller) ─────────────────────────────────────────────

/// Commands sent from the UI thread to the async controller.
#[derive(Debug)]
pub enum AppCommand {
    // ── Instance lifecycle ──
    CreateInstance {
        task_id: String,
        ver: VersionInfo,
        name: String,
        loader: Option<(String, String)>,
        config: LauncherConfig,
    },
    LaunchInstance {
        idx: usize,
        instance: Instance,
        config: LauncherConfig,
    },
    ExportInstance {
        instance: Instance,
        config: LauncherConfig,
    },
    ImportMrpack {
        task_id: String,
        config: LauncherConfig,
    },

    // ── Auth ──
    StartMsLogin {
        client_id: String,
        config: LauncherConfig,
    },
    StartAuthlibLogin {
        server_url: String,
        email: String,
        password: String,
    },

    // ── Java ──
    DownloadJava {
        task_id: String,
        required_major: u32,
        java_dir: PathBuf,
        launch_idx: usize,
    },

    // ── Versions & Loaders ──
    FetchVersionManifest {
        mirror: DownloadMirror,
    },
    FetchLoaderVersions {
        mc_version: String,
    },

    // ── Mods ──
    SearchMods {
        query: String,
        mc_version: String,
        loader: Option<String>,
        offset: u32,
    },
    LoadModVersions {
        slug: String,
        mc_version: String,
        loader: Option<String>,
    },
    ResolveDeps {
        slug: String,
        mc_version: String,
        loader: String,
        instance_dir: PathBuf,
    },
    InstallMod {
        pending: PendingModInstall,
        include_deps: bool,
    },

    // ── CurseForge ──
    CfSearchMods {
        query: String,
        mc_version: String,
        loader: Option<String>,
        api_key: String,
        index: u32,
    },
    CfLoadFiles {
        mod_id: u32,
        mc_version: String,
        loader: Option<String>,
        api_key: String,
    },
    CfResolveDeps {
        mod_id: u32,
        mc_version: String,
        loader: String,
        instance_dir: PathBuf,
        api_key: String,
    },
    CfInstallMod {
        mod_id: u32,
        mc_version: String,
        loader: String,
        instance_dir: PathBuf,
        api_key: String,
    },

    // ── Mod updates ──
    CheckModUpdates {
        mods_dir: PathBuf,
        mc_version: String,
        loader: String,
    },

    // ── Misc ──
    CheckForUpdates,
    CancelTask {
        task_id: String,
    },
}

// ─── Events (Controller → UI) ───────────────────────────────────────────────

/// Events sent from the async controller back to the UI thread.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // ── Versions ──
    VersionsFetched {
        all_versions: Vec<VersionInfo>,
        releases: Vec<VersionInfo>,
    },
    LoaderVersionsFetched {
        versions: HashMap<ModLoaderType, Vec<ModLoaderVersion>>,
    },
    LoaderFetchFailed,

    // ── Install progress ──
    InstallStatus(String),
    InstallProgress {
        task_id: String,
        completed: usize,
        total: usize,
        label: String,
    },
    InstallFinished {
        task_id: String,
        success: bool,
        message: String,
    },

    // ── Auth ──
    DeviceCode(DeviceCodeResponse),
    LoginComplete {
        account: AuthMethod,
    },
    LoginFailed(String),

    // ── Java ──
    JavaProgress(String),
    JavaInstalled {
        launch_idx: usize,
    },
    JavaFailed(String),

    // ── Mods ──
    ModSearchResults {
        hits: Vec<SearchHit>,
        total_hits: u32,
    },
    ModVersions(Vec<ProjectVersion>),
    ModPendingInstall(PendingModInstall),
    ModInstalled {
        count: usize,
    },
    ModError(String),

    // ── CurseForge ──
    CfSearchResults {
        mods: Vec<miao_core::curseforge::api::CfMod>,
        total_count: u32,
    },
    CfFileVersions(Vec<miao_core::curseforge::api::CfFile>),
    CfPendingInstall {
        mod_name: String,
        deps: Vec<miao_core::curseforge::api::CfResolvedDep>,
        instance_dir: PathBuf,
        mc_version: String,
        loader: String,
        api_key: String,
    },
    CfInstalled {
        count: usize,
    },
    CfError(String),

    ModUpdatesResult {
        updates: Vec<miao_core::modrinth::api::ModUpdateInfo>,
    },

    // ── Game log ──
    GameLogLine(String),
    GameExited {
        exit_code: Option<i32>,
    },

    // ── Instance ──
    ExportResult(String),
    ImportResult {
        success: bool,
        message: String,
    },

    // ── Misc ──
    UpdateAvailable(String),
    TaskCancelled {
        task_id: String,
    },
    Error(String),
}
