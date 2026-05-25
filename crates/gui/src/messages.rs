//! Message types for UI ↔ Controller communication.
//!
//! `AppCommand` flows from UI to the async controller.
//! `AppEvent` flows from the controller back to the UI.

use std::collections::HashMap;
use std::path::PathBuf;

use miao_core::auth::microsoft::DeviceCodeResponse;
use miao_core::auth::AuthMethod;
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
        config: LauncherConfig,
    },

    // ── Auth ──
    StartMsLogin {
        client_id: String,
        config: LauncherConfig,
    },

    // ── Java ──
    DownloadJava {
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

    // ── Misc ──
    CheckForUpdates,
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
        completed: usize,
        total: usize,
        label: String,
    },
    InstallFinished {
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
    ModSearchResults(Vec<SearchHit>),
    ModVersions(Vec<ProjectVersion>),
    ModPendingInstall(PendingModInstall),
    ModInstalled {
        count: usize,
    },
    ModError(String),

    // ── Game log ──
    GameLogLine(String),
    GameExited,

    // ── Instance ──
    ExportResult(String),
    ImportResult {
        success: bool,
        message: String,
    },

    // ── Misc ──
    UpdateAvailable(String),
    Error(String),
}
