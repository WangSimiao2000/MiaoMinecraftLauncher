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
        task_id: String,
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

    // ── Modpack source ──
    FetchModpackManifest {
        source_id: String,
        manifest_url: String,
    },
    ResolveModpack {
        source_id: String,
        pack_id: String,
        pack_url: String,
        mc_version: String,
        loader: String,
        instance_name: String,
        config: LauncherConfig,
    },
    ApplyModpackInstall {
        source_id: String,
        pack_id: String,
        pack_url: String,
        instance_name: String,
        config: Box<LauncherConfig>,
        report: Box<miao_core::modpack_source::ResolutionReport>,
        pack_raw: Vec<u8>,
        pack: Box<miao_core::modpack_source::Pack>,
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
    /// Re-fetch and re-cache the avatar/cape for an account by UUID. The launcher
    /// only auto-fetches skins on first login, so cache loss (e.g. user wiped data
    /// dir) leaves the GUI rendering placeholder PNGs forever. This command lets us
    /// rebuild the cache eagerly on startup.
    RefreshSkin {
        /// Compact-form UUID (no hyphens) — matches the on-disk cache file name.
        uuid: String,
        data_dir: PathBuf,
    },

    // ── Java ──
    DownloadJava {
        task_id: String,
        required_major: u32,
        launch_idx: usize,
        config: LauncherConfig,
    },
    /// Trigger a background Java detection scan. The result is delivered via
    /// [`AppEvent::JavaDetected`]. If `force` is false, a recent cache may be reused.
    RefreshJava {
        data_dir: PathBuf,
        force: bool,
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
        failed: Vec<ModLoaderType>,
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

    // ── Modpack source ──
    ModpackManifestFetched {
        source_id: String,
        manifest: miao_core::modpack_source::Manifest,
    },
    ModpackManifestFailed {
        source_id: String,
        error: String,
    },
    ModpackResolutionReady {
        source_id: String,
        pack_id: String,
        pack_url: String,
        instance_name: String,
        config: Box<LauncherConfig>,
        report: Box<miao_core::modpack_source::ResolutionReport>,
        pack_raw: Vec<u8>,
        pack: Box<miao_core::modpack_source::Pack>,
    },
    ModpackResolutionFailed {
        #[allow(dead_code)]
        source_id: String,
        #[allow(dead_code)]
        pack_id: String,
        error: String,
    },
    ModpackInstallFinished {
        task_id: String,
        #[allow(dead_code)]
        source_id: String,
        #[allow(dead_code)]
        pack_id: String,
        success: bool,
        message: String,
    },

    // ── Auth ──
    DeviceCode(DeviceCodeResponse),
    LoginComplete {
        account: AuthMethod,
    },
    LoginFailed(String),
    /// Sent after a [`AppCommand::RefreshSkin`] task completes (whether or not the
    /// fetch succeeded — the UI just needs to drop any cached image so the new
    /// PNG on disk gets re-read).
    SkinRefreshed {
        /// Compact-form UUID matching the cache file name.
        uuid: String,
    },

    // ── Java ──
    JavaInstalled {
        launch_idx: usize,
    },
    JavaFailed(String),
    /// Result of a background Java detection scan.
    JavaDetected {
        installations: Vec<miao_core::java::JavaInstallation>,
    },

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
    /// Emitted by the launch worker exactly once when the JVM has been spawned
    /// successfully. Unlike [`AppEvent::InstallFinished`] this carries no
    /// side-effects (no instance list refresh, no tab switching, no Java
    /// re-detection) — its sole purpose is to clear the launch-progress entry
    /// from `active_installs` / `install_progress` so the bottom-bar spinner
    /// stops. The actual game-running indicator is then driven by
    /// `game_log.running` (set by [`AppEvent::GameLogLine`] arrivals).
    LaunchSpawned {
        task_id: String,
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
}
