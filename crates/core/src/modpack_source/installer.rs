use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::instance::{Instance, ModpackSubscription};
use crate::modpack_source::manifest::{ConfigOverlay, OverlayFile, Pack};
use crate::modpack_source::resolver::{
    ResolutionReport, ResolvedMod, ResolvedOverlay, ResolvedStatus,
};
use crate::version::install::{EnsurePhase, EnsureProgress};

pub trait InstallProgressSink: Send + Sync {
    fn on_phase(&self, phase: InstallPhase);
    fn on_mod_progress(&self, completed: usize, total: usize, current_name: &str);
    fn on_overlay_progress(&self, completed: usize, total: usize, current_path: &str);
    fn on_minecraft_progress(&self, completed: usize, total: usize, label: &str) {
        let _ = (completed, total, label);
    }
}

#[derive(Debug, Clone)]
pub enum InstallPhase {
    DownloadingMinecraft {
        mc: String,
    },
    InstallingLoader {
        mc: String,
        loader: String,
        version: Option<String>,
    },
    DownloadingMods {
        total: usize,
    },
    WritingOverlay {
        total: usize,
    },
    Finalizing,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("instance directory '{name}' already exists; refusing to overwrite")]
    InstanceAlreadyExists { name: String },

    #[error("resolution has unresolved core mods or hard incompatibility; cannot install")]
    PreCheckFailed { reason: String },

    #[error("download failed for {url}: {reason}")]
    Download { url: String, reason: String },

    #[error("hash mismatch for {target} (expected {expected}, got {actual})")]
    HashMismatch {
        target: String,
        expected: String,
        actual: String,
    },

    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("loader install failed: {0}")]
    Loader(String),

    #[error("vanilla Minecraft install failed: {0}")]
    Minecraft(String),

    #[error("rollback failed: original error {original}; cleanup error {cleanup}")]
    RollbackFailed { original: String, cleanup: String },
}

#[async_trait]
pub trait InstallExecutor: Send + Sync {
    async fn download_to(&self, url: &str, dest: &Path) -> Result<(), String>;

    async fn install_loader(
        &self,
        instance_dir: &Path,
        loader_type: &str,
        mc_version: &str,
        loader_version: Option<&str>,
    ) -> Result<crate::instance::ModLoaderConfig, String>;

    async fn ensure_minecraft_installed(
        &self,
        mc_version: &str,
        progress: Option<EnsureProgress>,
    ) -> Result<(), String> {
        let _ = (mc_version, progress);
        Ok(())
    }
}

pub struct InstallPlan<'a> {
    pub instance_name: String,
    pub pack: &'a Pack,
    pub pack_raw: &'a [u8],
    pub report: &'a ResolutionReport,
    pub source_id: String,
    pub source_url: String,
    pub manifest_etag: Option<String>,
    pub progress: Option<Arc<dyn InstallProgressSink>>,
}

pub struct InstallOutcome {
    pub instance: Instance,
    pub overlays_installed: Vec<ResolvedOverlay>,
}

pub async fn install(
    plan: InstallPlan<'_>,
    instances_root: &Path,
    executor: &dyn InstallExecutor,
) -> Result<InstallOutcome, InstallError> {
    let instance_dir = instances_root.join(&plan.instance_name);
    if instance_dir.exists() {
        return Err(InstallError::InstanceAlreadyExists {
            name: plan.instance_name.clone(),
        });
    }

    enforce_pre_check(plan.report)?;

    Instance::create_directories(&instance_dir).map_err(|e| InstallError::Io(io_err(e)))?;

    let staging = instance_dir.join(".staging");
    std::fs::create_dir_all(&staging)?;

    let result = run_install(&plan, &instance_dir, &staging, executor).await;

    match result {
        Ok(outcome) => {
            let _ = std::fs::remove_dir_all(&staging);
            Ok(outcome)
        }
        Err(e) => {
            let cleanup = std::fs::remove_dir_all(&instance_dir);
            if let Err(c) = cleanup {
                return Err(InstallError::RollbackFailed {
                    original: e.to_string(),
                    cleanup: c.to_string(),
                });
            }
            Err(e)
        }
    }
}

async fn run_install(
    plan: &InstallPlan<'_>,
    instance_dir: &Path,
    staging: &Path,
    executor: &dyn InstallExecutor,
) -> Result<InstallOutcome, InstallError> {
    let loader_type_str = plan.pack.loader.canonical();
    let loader_version = plan.report.loader_version.as_deref();

    if let Some(sink) = &plan.progress {
        sink.on_phase(InstallPhase::DownloadingMinecraft {
            mc: plan.report.mc_version.clone(),
        });
    }
    let ensure_progress: Option<EnsureProgress> = plan.progress.as_ref().map(|sink| {
        let sink = Arc::clone(sink);
        Arc::new(move |phase: EnsurePhase, completed: usize, total: usize| {
            let label = match phase {
                EnsurePhase::FetchingManifest => "Fetching version manifest".to_string(),
                EnsurePhase::DownloadingClient { .. } => {
                    format!("Downloading libraries ({completed}/{total})")
                }
                EnsurePhase::DownloadingAssets { .. } => {
                    format!("Downloading assets ({completed}/{total})")
                }
                EnsurePhase::ExtractingNatives => "Extracting natives".to_string(),
            };
            sink.on_minecraft_progress(completed, total, &label);
        }) as EnsureProgress
    });
    executor
        .ensure_minecraft_installed(&plan.report.mc_version, ensure_progress)
        .await
        .map_err(InstallError::Minecraft)?;

    if let Some(sink) = &plan.progress {
        sink.on_phase(InstallPhase::InstallingLoader {
            mc: plan.report.mc_version.clone(),
            loader: loader_type_str.to_string(),
            version: loader_version.map(|s| s.to_string()),
        });
    }

    let loader_config = executor
        .install_loader(
            instance_dir,
            loader_type_str,
            &plan.report.mc_version,
            loader_version,
        )
        .await
        .map_err(InstallError::Loader)?;

    let staging_mods = staging.join("mods");
    std::fs::create_dir_all(&staging_mods)?;

    let downloadable: Vec<&ResolvedMod> = plan
        .report
        .mods
        .iter()
        .filter(|m| matches!(m.status, ResolvedStatus::Compatible))
        .collect();
    let mod_total = downloadable.len();
    if let Some(sink) = &plan.progress {
        sink.on_phase(InstallPhase::DownloadingMods { total: mod_total });
    }

    for (idx, m) in downloadable.iter().enumerate() {
        if let Some(sink) = &plan.progress {
            sink.on_mod_progress(idx, mod_total, &m.display_name);
        }
        download_mod(executor, m, &staging_mods).await?;
    }
    if let Some(sink) = &plan.progress {
        sink.on_mod_progress(mod_total, mod_total, "");
    }

    let mods_dir = Instance::mods_dir(instance_dir);
    move_directory(&staging_mods, &mods_dir)?;

    let mut overlays_installed: Vec<ResolvedOverlay> = Vec::new();
    if let Some(overlay) = &plan.pack.config_overlay {
        let applicable: Vec<&OverlayFile> = overlay
            .files
            .iter()
            .filter(|f| applies_to(f, &plan.report.mc_version))
            .collect();
        if let Some(sink) = &plan.progress {
            sink.on_phase(InstallPhase::WritingOverlay {
                total: applicable.len(),
            });
        }
        let staging_overlay = staging.join("overlay");
        std::fs::create_dir_all(&staging_overlay)?;
        overlays_installed = stage_and_install_overlay(
            overlay,
            &plan.report.mc_version,
            &staging_overlay,
            instance_dir,
            executor,
            plan.progress.as_ref(),
        )
        .await?;
    }

    if let Some(sink) = &plan.progress {
        sink.on_phase(InstallPhase::Finalizing);
    }

    let metadata_dir = instance_dir.join(".miao-modpack");
    std::fs::create_dir_all(&metadata_dir)?;
    std::fs::write(metadata_dir.join("pack.json"), plan.pack_raw)?;

    let report_with_overlays = ResolutionReport {
        overlays: overlays_installed.clone(),
        ..plan.report.clone()
    };
    std::fs::write(
        metadata_dir.join("resolved.json"),
        serde_json::to_vec_pretty(&report_with_overlays)
            .map_err(|e| InstallError::Io(io_err(e)))?,
    )?;

    let source_record = SourceRecord {
        source_id: plan.source_id.clone(),
        source_url: plan.source_url.clone(),
        manifest_etag: plan.manifest_etag.clone(),
        subscribed_at: chrono::Utc::now(),
    };
    std::fs::write(
        metadata_dir.join("source.json"),
        serde_json::to_vec_pretty(&source_record).map_err(|e| InstallError::Io(io_err(e)))?,
    )?;

    let mut instance = Instance::new(&plan.instance_name, &plan.report.mc_version);
    instance.mod_loader = Some(loader_config);
    instance.modpack_subscription = Some(ModpackSubscription {
        source_id: plan.source_id.clone(),
        pack_id: plan.pack.id.clone(),
        pack_version: plan.pack.version.clone(),
        pack_content_hash: plan.report.pack_content_hash.clone(),
        mc_version: plan.report.mc_version.clone(),
        installed_at: chrono::Utc::now(),
    });
    instance
        .save_to(instance_dir)
        .map_err(|e| InstallError::Io(io_err(e)))?;

    Ok(InstallOutcome {
        instance,
        overlays_installed,
    })
}

fn enforce_pre_check(report: &ResolutionReport) -> Result<(), InstallError> {
    if let Some(cycle) = &report.cycle
        && !cycle.is_empty()
    {
        return Err(InstallError::PreCheckFailed {
            reason: "dependency cycle in resolution; resolver should not have produced this"
                .to_string(),
        });
    }
    for m in &report.mods {
        match (&m.status, m.criticality) {
            (
                ResolvedStatus::Conflict { reason },
                crate::modpack_source::manifest::Criticality::Core,
            ) => {
                return Err(InstallError::PreCheckFailed {
                    reason: format!("core mod {} is in Conflict: {}", m.project_id, reason),
                });
            }
            (
                ResolvedStatus::Abandoned { .. },
                crate::modpack_source::manifest::Criticality::Core,
            ) => {
                return Err(InstallError::PreCheckFailed {
                    reason: format!("core mod {} is Abandoned", m.project_id),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

async fn download_mod(
    executor: &dyn InstallExecutor,
    m: &ResolvedMod,
    staging_mods: &Path,
) -> Result<(), InstallError> {
    let url = m
        .file_url
        .as_deref()
        .ok_or_else(|| InstallError::Download {
            url: String::new(),
            reason: format!("Compatible mod {} missing file_url", m.project_id),
        })?;
    let file_name = m
        .file_name
        .as_deref()
        .ok_or_else(|| InstallError::Download {
            url: url.to_string(),
            reason: format!("Compatible mod {} missing file_name", m.project_id),
        })?;

    let dest = staging_mods.join(file_name);
    executor
        .download_to(url, &dest)
        .await
        .map_err(|e| InstallError::Download {
            url: url.to_string(),
            reason: e,
        })?;

    if let Some(expected) = &m.file_sha512 {
        verify_hash(&dest, expected, HashKind::Sha512)?;
    } else if let Some(expected) = &m.file_sha1 {
        verify_hash(&dest, expected, HashKind::Sha1)?;
    }

    Ok(())
}

async fn stage_and_install_overlay(
    overlay: &ConfigOverlay,
    mc_version: &str,
    staging_overlay: &Path,
    instance_dir: &Path,
    executor: &dyn InstallExecutor,
    progress: Option<&Arc<dyn InstallProgressSink>>,
) -> Result<Vec<ResolvedOverlay>, InstallError> {
    let applicable: Vec<&OverlayFile> = overlay
        .files
        .iter()
        .filter(|f| applies_to(f, mc_version))
        .collect();
    let total = applicable.len();
    let mut installed = Vec::new();
    for (idx, f) in applicable.iter().enumerate() {
        if let Some(sink) = progress {
            sink.on_overlay_progress(idx, total, &f.target);
        }
        let url = format!(
            "{base}/{path}",
            base = overlay.base_url.trim_end_matches('/'),
            path = f.path
        );
        let staged_path = staging_overlay.join(file_name_only(&f.target));
        if let Some(parent) = staged_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        executor
            .download_to(&url, &staged_path)
            .await
            .map_err(|e| InstallError::Download {
                url: url.clone(),
                reason: e,
            })?;
        verify_hash(&staged_path, &f.sha256, HashKind::Sha256)?;

        let final_path = instance_dir.join(&f.target);
        if let Some(parent) = final_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        rename_with_exdev_fallback(&staged_path, &final_path)?;

        let actual_size = std::fs::metadata(&final_path)?.len();
        installed.push(ResolvedOverlay {
            target: f.target.clone(),
            source_url: url,
            sha256: f.sha256.clone(),
            original_size: f.original_size.unwrap_or(actual_size),
            preserve: f.preserve,
            installed_at: chrono::Utc::now(),
        });
    }
    if let Some(sink) = progress {
        sink.on_overlay_progress(total, total, "");
    }
    Ok(installed)
}

fn applies_to(f: &OverlayFile, mc_version: &str) -> bool {
    match &f.applies_to_mc {
        None => true,
        Some(list) => list.iter().any(|v| v == mc_version),
    }
}

fn file_name_only(target: &str) -> String {
    target.replace(['/', '\\'], "_")
}

fn move_directory(src: &Path, dst: &Path) -> Result<(), InstallError> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        rename_with_exdev_fallback(&entry.path(), &target)?;
    }
    let _ = std::fs::remove_dir(src);
    Ok(())
}

fn rename_with_exdev_fallback(src: &Path, dst: &Path) -> Result<(), InstallError> {
    match std::fs::rename(src, dst) {
        Ok(()) => Ok(()),
        Err(e) if is_cross_device(&e) => {
            tracing::warn!(?src, ?dst, "EXDEV: falling back to copy + remove");
            std::fs::copy(src, dst)?;
            std::fs::remove_file(src)?;
            Ok(())
        }
        Err(e) => Err(InstallError::Io(e)),
    }
}

fn is_cross_device(e: &std::io::Error) -> bool {
    e.raw_os_error() == Some(libc_exdev())
}

#[cfg(target_os = "linux")]
fn libc_exdev() -> i32 {
    18
}

#[cfg(not(target_os = "linux"))]
fn libc_exdev() -> i32 {
    -1
}

#[derive(Debug, Clone, Copy)]
enum HashKind {
    Sha1,
    Sha256,
    Sha512,
}

fn verify_hash(path: &Path, expected: &str, kind: HashKind) -> Result<(), InstallError> {
    use sha1::Sha1;
    use sha2::Sha512;

    let bytes = std::fs::read(path)?;
    let actual = match kind {
        HashKind::Sha1 => {
            let mut h = Sha1::new();
            h.update(&bytes);
            hex(h.finalize())
        }
        HashKind::Sha256 => {
            let mut h = Sha256::new();
            h.update(&bytes);
            hex(h.finalize())
        }
        HashKind::Sha512 => {
            let mut h = Sha512::new();
            h.update(&bytes);
            hex(h.finalize())
        }
    };
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(InstallError::HashMismatch {
            target: path.to_string_lossy().to_string(),
            expected: expected.to_string(),
            actual,
        })
    }
}

fn hex<T: AsRef<[u8]>>(bytes: T) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn io_err<E: std::error::Error>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SourceRecord {
    source_id: String,
    source_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_etag: Option<String>,
    subscribed_at: chrono::DateTime<chrono::Utc>,
}

pub fn cleanup_orphan_staging(instance_dir: &Path) -> std::io::Result<()> {
    let staging = instance_dir.join(".staging");
    if staging.exists() {
        std::fs::remove_dir_all(staging)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modpack_source::manifest::{
        Criticality, Loader, ModPolicy, ModSource, OverlayFile, PackMod,
    };
    use crate::modpack_source::resolver::{ConflictReport, ResolvedStatus};
    use sha1::Sha1;
    use sha2::Sha512;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct FakeExecutor {
        files: Mutex<HashMap<String, Vec<u8>>>,
        loader_returns: Mutex<Option<crate::instance::ModLoaderConfig>>,
        loader_should_fail: Mutex<bool>,
    }

    impl FakeExecutor {
        fn new() -> Self {
            Self {
                files: Mutex::new(HashMap::new()),
                loader_returns: Mutex::new(None),
                loader_should_fail: Mutex::new(false),
            }
        }

        fn put(&self, url: &str, body: Vec<u8>) {
            self.files.lock().unwrap().insert(url.to_string(), body);
        }
    }

    #[async_trait]
    impl InstallExecutor for FakeExecutor {
        async fn download_to(&self, url: &str, dest: &Path) -> Result<(), String> {
            let body = self
                .files
                .lock()
                .unwrap()
                .get(url)
                .cloned()
                .ok_or_else(|| format!("404 for {url}"))?;
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(dest, &body).map_err(|e| e.to_string())?;
            Ok(())
        }

        async fn install_loader(
            &self,
            _instance_dir: &Path,
            loader_type: &str,
            mc_version: &str,
            _loader_version: Option<&str>,
        ) -> Result<crate::instance::ModLoaderConfig, String> {
            if *self.loader_should_fail.lock().unwrap() {
                return Err("simulated loader failure".to_string());
            }
            if let Some(cfg) = self.loader_returns.lock().unwrap().clone() {
                return Ok(cfg);
            }
            Ok(crate::instance::ModLoaderConfig {
                loader_type: match loader_type {
                    "fabric" => crate::modloader::ModLoaderType::Fabric,
                    "forge" => crate::modloader::ModLoaderType::Forge,
                    "neoforge" => crate::modloader::ModLoaderType::NeoForge,
                    "quilt" => crate::modloader::ModLoaderType::Quilt,
                    _ => crate::modloader::ModLoaderType::Fabric,
                },
                version: format!("test-loader-for-{mc_version}"),
                main_class: None,
                extra_libraries: Vec::new(),
            })
        }
    }

    fn pack_minimal() -> Pack {
        Pack {
            pack_format: "miao:1.0.0".to_string(),
            id: "p1".to_string(),
            display_name: "Pack 1".to_string(),
            version: "0.1.0".to_string(),
            summary: None,
            description: None,
            loader: Loader::Fabric,
            mc_versions: vec!["1.21.5".to_string()],
            loader_versions: HashMap::new(),
            mods: vec![PackMod {
                source: ModSource::Modrinth,
                id: "sodium".to_string(),
                name: "Sodium".to_string(),
                policy: ModPolicy::Auto,
                locked_version: None,
                criticality: Criticality::Core,
                deprecated_after: None,
                replacement: None,
            }],
            config_overlay: None,
        }
    }

    fn report_compatible_with(file_url: &str, file_name: &str, body: &[u8]) -> ResolutionReport {
        let mut sha1 = Sha1::new();
        sha1.update(body);
        let sha1_hex = hex(sha1.finalize());
        let mut sha512 = Sha512::new();
        sha512.update(body);
        let sha512_hex = hex(sha512.finalize());
        ResolutionReport {
            pack_id: "p1".to_string(),
            pack_version: "0.1.0".to_string(),
            pack_content_hash: "deadbeef".to_string(),
            mc_version: "1.21.5".to_string(),
            loader: Loader::Fabric,
            loader_version: Some("0.16.10".to_string()),
            resolved_at: chrono::Utc::now(),
            mods: vec![ResolvedMod {
                source: ModSource::Modrinth,
                project_id: "sodium".to_string(),
                display_name: "Sodium".to_string(),
                criticality: Criticality::Core,
                status: ResolvedStatus::Compatible,
                version_id: Some("v1".to_string()),
                file_url: Some(file_url.to_string()),
                file_sha1: Some(sha1_hex),
                file_sha512: Some(sha512_hex),
                file_size: Some(body.len() as u64),
                file_name: Some(file_name.to_string()),
                auto_added_for: None,
                replacement_hint: None,
            }],
            overlays: Vec::new(),
            conflicts: Vec::new(),
            cycle: None,
        }
    }

    #[tokio::test]
    async fn t_install_01_happy_path_creates_instance_with_mod() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();
        let body = b"mock-mod-jar-bytes";
        exec.put("https://example.com/sodium.jar", body.to_vec());

        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", body);

        let plan = InstallPlan {
            instance_name: "test1".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let outcome = install(plan, instances_root.path(), &exec).await.unwrap();
        assert_eq!(outcome.instance.name, "test1");
        let instance_dir = instances_root.path().join("test1");
        assert!(instance_dir.join("mods/sodium.jar").exists());
        assert!(instance_dir.join(".miao-modpack/pack.json").exists());
        assert!(instance_dir.join(".miao-modpack/resolved.json").exists());
        assert!(instance_dir.join(".miao-modpack/source.json").exists());
        assert!(instance_dir.join("instance.toml").exists());
        assert!(!instance_dir.join(".staging").exists());

        let loaded = Instance::load_from(&instance_dir).unwrap();
        let sub = loaded.modpack_subscription.expect("present");
        assert_eq!(sub.source_id, "miao");
        assert_eq!(sub.pack_id, "p1");
        assert_eq!(sub.pack_content_hash, "deadbeef");
    }

    #[tokio::test]
    async fn t_install_02_existing_instance_name_rejected_pre_check() {
        let instances_root = tempfile::tempdir().unwrap();
        let conflicting = instances_root.path().join("test2");
        std::fs::create_dir_all(&conflicting).unwrap();
        std::fs::write(conflicting.join("user-savefile"), b"important data").unwrap();

        let exec = FakeExecutor::new();
        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let body = b"jar";
        exec.put("https://example.com/sodium.jar", body.to_vec());
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", body);

        let plan = InstallPlan {
            instance_name: "test2".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(
            result,
            Err(InstallError::InstanceAlreadyExists { .. })
        ));
        assert!(conflicting.join("user-savefile").exists());
    }

    #[tokio::test]
    async fn t_install_03_mod_download_404_rolls_back_full_instance() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();

        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let body = b"jar";
        let report = report_compatible_with("https://example.com/missing.jar", "missing.jar", body);

        let plan = InstallPlan {
            instance_name: "test3".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(result, Err(InstallError::Download { .. })));
        assert!(!instances_root.path().join("test3").exists());
    }

    #[tokio::test]
    async fn t_install_04_mod_sha512_mismatch_rolls_back() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();
        exec.put(
            "https://example.com/sodium.jar",
            b"GARBAGE-not-the-expected-bytes".to_vec(),
        );

        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let report =
            report_compatible_with("https://example.com/sodium.jar", "sodium.jar", b"original");

        let plan = InstallPlan {
            instance_name: "test4".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(result, Err(InstallError::HashMismatch { .. })));
        assert!(!instances_root.path().join("test4").exists());
    }

    #[tokio::test]
    async fn t_install_05_overlay_sha256_mismatch_rolls_back() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();

        let body = b"jar";
        exec.put("https://example.com/sodium.jar", body.to_vec());
        exec.put(
            "https://example.com/cfg/options.txt",
            b"BADCONTENT".to_vec(),
        );

        let mut pack = pack_minimal();
        pack.config_overlay = Some(ConfigOverlay {
            base_url: "https://example.com/cfg".to_string(),
            files: vec![OverlayFile {
                path: "options.txt".to_string(),
                target: "options.txt".to_string(),
                sha256: "0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
                applies_to_mc: None,
                original_size: None,
                preserve: false,
            }],
        });

        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", body);

        let plan = InstallPlan {
            instance_name: "test5".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(result, Err(InstallError::HashMismatch { .. })));
        assert!(!instances_root.path().join("test5").exists());
    }

    #[tokio::test]
    async fn t_install_06_loader_failure_rolls_back() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();
        *exec.loader_should_fail.lock().unwrap() = true;

        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let body = b"jar";
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", body);

        let plan = InstallPlan {
            instance_name: "test6".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(result, Err(InstallError::Loader(_))));
        assert!(!instances_root.path().join("test6").exists());
    }

    #[tokio::test]
    async fn t_install_07_pre_check_blocks_when_core_mod_in_conflict() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();

        let pack = pack_minimal();
        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let mut report =
            report_compatible_with("https://example.com/sodium.jar", "sodium.jar", b"jar");
        report.mods[0].status = ResolvedStatus::Conflict {
            reason: "test".to_string(),
        };

        let plan = InstallPlan {
            instance_name: "test7".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let result = install(plan, instances_root.path(), &exec).await;
        assert!(matches!(result, Err(InstallError::PreCheckFailed { .. })));
        assert!(!instances_root.path().join("test7").exists());
    }

    #[tokio::test]
    async fn t_install_08_overlays_with_preserve_persisted_to_resolved_json() {
        let instances_root = tempfile::tempdir().unwrap();
        let exec = FakeExecutor::new();

        let body = b"jar";
        exec.put("https://example.com/sodium.jar", body.to_vec());
        let cfg_body = b"sodium-options=fast";
        exec.put("https://example.com/cfg/options.txt", cfg_body.to_vec());

        let mut sha = Sha256::new();
        sha.update(cfg_body);
        let sha256_hex = hex(sha.finalize());

        let mut pack = pack_minimal();
        pack.config_overlay = Some(ConfigOverlay {
            base_url: "https://example.com/cfg".to_string(),
            files: vec![OverlayFile {
                path: "options.txt".to_string(),
                target: "options.txt".to_string(),
                sha256: sha256_hex,
                applies_to_mc: None,
                original_size: Some(cfg_body.len() as u64),
                preserve: true,
            }],
        });

        let pack_raw = serde_json::to_vec(&pack).unwrap();
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", body);

        let plan = InstallPlan {
            instance_name: "test8".to_string(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: "miao".to_string(),
            source_url: "https://example.com/manifest.json".to_string(),
            manifest_etag: None,
            progress: None,
        };

        let outcome = install(plan, instances_root.path(), &exec).await.unwrap();
        let overlays = outcome.overlays_installed;
        assert_eq!(overlays.len(), 1);
        assert!(overlays[0].preserve);
        assert_eq!(overlays[0].original_size, cfg_body.len() as u64);

        let resolved_path = instances_root
            .path()
            .join("test8/.miao-modpack/resolved.json");
        let raw = std::fs::read(&resolved_path).unwrap();
        let parsed: ResolutionReport = serde_json::from_slice(&raw).unwrap();
        assert_eq!(parsed.overlays.len(), 1);
        assert!(parsed.overlays[0].preserve);
    }

    #[tokio::test]
    async fn t_install_09_cleanup_orphan_staging_removes_dir() {
        let dir = tempfile::tempdir().unwrap();
        let instance_dir = dir.path().join("inst");
        std::fs::create_dir_all(instance_dir.join(".staging/mods")).unwrap();
        std::fs::write(instance_dir.join(".staging/mods/x.jar"), b"x").unwrap();

        cleanup_orphan_staging(&instance_dir).unwrap();

        assert!(!instance_dir.join(".staging").exists());
        assert!(instance_dir.exists());
    }

    #[test]
    fn t_install_10_pre_check_passes_for_compatible_only_report() {
        let report = report_compatible_with("https://example.com/sodium.jar", "sodium.jar", b"jar");
        assert!(enforce_pre_check(&report).is_ok());
    }

    #[test]
    fn t_install_11_pre_check_passes_for_optional_conflict() {
        let mut report =
            report_compatible_with("https://example.com/sodium.jar", "sodium.jar", b"jar");
        report.mods[0].criticality = Criticality::Optional;
        report.mods[0].status = ResolvedStatus::Conflict {
            reason: "test".to_string(),
        };
        assert!(enforce_pre_check(&report).is_ok());
    }

    #[test]
    fn t_install_12_unused_conflict_report_referenced() {
        let _ = ConflictReport {
            a_project_id: "a".to_string(),
            b_project_id: "b".to_string(),
            reason: "x".to_string(),
        };
    }
}
