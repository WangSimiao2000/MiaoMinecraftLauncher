//! Source-neutral Java installer.
//!
//! Defines [`JavaInstallPlan`], a description of "what files need to land where" that
//! is independent of which upstream produced it, and an [`execute`] routine that turns
//! a plan into an installed JRE.
//!
//! New sources implement a function with the signature
//! `async fn(&Client, &LauncherConfig, u32) -> Result<JavaInstallPlan>` and add a
//! variant to [`crate::config::JavaSource`]. Dispatch happens in [`plan`] below — the
//! GUI/CLI never need to know which source they're using.

use std::path::{Path, PathBuf};

use crate::config::{JavaSource, LauncherConfig};
use crate::download::DownloadTask;
use crate::download::manager::DownloadManager;
use crate::error::{MiaoError, Result};

use super::extract::ArchiveFormat;

/// One archive download + post-extract step. After the archive lands at `archive_dest`
/// it is extracted into [`JavaInstallPlan::install_dir`] with the top
/// `strip_components` path components removed.
#[derive(Debug, Clone)]
pub struct ArchiveTask {
    pub task: DownloadTask,
    pub archive_dest: PathBuf,
    pub format: ArchiveFormat,
    pub strip_components: usize,
}

/// Source-neutral plan for installing a Java runtime. A plan can mix per-file Mojang
/// downloads with archive-based sources (Adoptium, Microsoft) — the executor handles
/// `tasks` first, then `archives`, then `links`/executables.
#[derive(Debug, Clone)]
pub struct JavaInstallPlan {
    pub variant: String,
    pub version: String,
    pub major: u32,
    pub source_name: &'static str,
    pub total_size: u64,
    pub file_count: usize,
    pub install_dir: PathBuf,
    pub tasks: Vec<DownloadTask>,
    pub archives: Vec<ArchiveTask>,
    pub links: Vec<(PathBuf, String)>,
    #[cfg(unix)]
    pub executables: Vec<PathBuf>,
}

/// Plan an install. Performs network I/O for source-specific manifest fetches but
/// downloads no JRE files.
pub async fn plan(
    http: &reqwest::Client,
    config: &LauncherConfig,
    required_major: u32,
) -> Result<JavaInstallPlan> {
    match config.java_source {
        JavaSource::Mojang => super::mojang::plan_install(http, config, required_major).await,
        JavaSource::Bmclapi => super::bmclapi::plan_install(http, config, required_major).await,
        JavaSource::Adoptium => super::adoptium::plan_install(http, config, required_major).await,
        JavaSource::Microsoft => super::microsoft::plan_install(http, config, required_major).await,
    }
}

/// Execute a plan: download all files (per-file + archives), extract archives, recreate
/// symlinks, set executable bits, and return the resulting `java`/`java.exe` path.
pub async fn execute(
    plan: JavaInstallPlan,
    config: &LauncherConfig,
    progress_cb: Option<crate::download::manager::ProgressCallback>,
) -> Result<PathBuf> {
    if plan.install_dir.exists() {
        std::fs::remove_dir_all(&plan.install_dir)?;
    }
    std::fs::create_dir_all(&plan.install_dir)?;

    let mut dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    );
    if let Some(cb) = progress_cb {
        dm = dm.with_progress_callback(cb);
    }

    let mut all_tasks = plan.tasks;
    all_tasks.extend(plan.archives.iter().map(|a| a.task.clone()));
    dm.download_all(all_tasks).await?;

    for archive in &plan.archives {
        super::extract::extract(
            &archive.archive_dest,
            &plan.install_dir,
            archive.format,
            archive.strip_components,
        )?;
        std::fs::remove_file(&archive.archive_dest).ok();
    }

    apply_links(&plan.links)?;

    #[cfg(unix)]
    apply_executable_bits(&plan.executables)?;

    let java_bin = java_binary_path(&plan.install_dir);
    if !java_bin.exists() {
        return Err(MiaoError::Other(format!(
            "Java install completed but binary not found at {}",
            java_bin.display()
        )));
    }
    Ok(java_bin)
}

fn apply_links(links: &[(PathBuf, String)]) -> Result<()> {
    for (link_path, target) in links {
        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if link_path.exists() {
            std::fs::remove_file(link_path).ok();
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link_path)?;
        }
        #[cfg(windows)]
        {
            // Windows symlinks need elevated permissions or developer mode. Fall back
            // to copying the underlying file so the JRE remains usable.
            let resolved = link_path
                .parent()
                .map(|p| p.join(target))
                .unwrap_or_else(|| PathBuf::from(target));
            if resolved.exists() {
                std::fs::copy(&resolved, link_path)?;
            } else {
                tracing::warn!(target: "miao::java",
                    "Skipping link {} -> {} (target missing)",
                    link_path.display(), target
                );
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn apply_executable_bits(paths: &[PathBuf]) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    for path in paths {
        if let Ok(meta) = std::fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            std::fs::set_permissions(path, perms)?;
        }
    }
    Ok(())
}

/// The path of the launcher binary inside an installed JRE directory.
pub fn java_binary_path(install_dir: &Path) -> PathBuf {
    let bin_dir = if std::env::consts::OS == "macos" {
        install_dir.join("jre.bundle/Contents/Home/bin")
    } else {
        install_dir.join("bin")
    };
    bin_dir.join(super::java_binary_name())
}

impl JavaSource {
    /// All sources, in display order. Used by the settings UI to enumerate options.
    pub fn all() -> &'static [JavaSource] {
        &[
            JavaSource::Mojang,
            JavaSource::Bmclapi,
            JavaSource::Adoptium,
            JavaSource::Microsoft,
        ]
    }

    pub fn id(&self) -> &'static str {
        match self {
            JavaSource::Mojang => "mojang",
            JavaSource::Bmclapi => "bmclapi",
            JavaSource::Adoptium => "adoptium",
            JavaSource::Microsoft => "microsoft",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            JavaSource::Mojang => "Mojang",
            JavaSource::Bmclapi => "BMCLAPI",
            JavaSource::Adoptium => "Adoptium Temurin",
            JavaSource::Microsoft => "Microsoft",
        }
    }
}

/// Helper used by source planners to enforce a consistent `install_dir` layout:
/// `<data_dir>/java/<source-id>/<variant>`. Keeping installs from different sources
/// in disjoint subdirectories prevents stale files from leaking between them.
pub fn install_dir_for(config: &LauncherConfig, source: JavaSource, variant: &str) -> PathBuf {
    config.data_dir.join("java").join(source.id()).join(variant)
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_source_default_is_mojang() {
        assert_eq!(JavaSource::default(), JavaSource::Mojang);
    }

    #[test]
    fn install_dir_segregates_by_source() {
        let config = LauncherConfig {
            data_dir: PathBuf::from("/tmp/test"),
            ..LauncherConfig::default()
        };
        let dir = install_dir_for(&config, JavaSource::Mojang, "java-runtime-gamma");
        assert!(dir.ends_with("java/mojang/java-runtime-gamma"));
    }

    #[test]
    fn java_source_all_includes_default() {
        assert!(JavaSource::all().contains(&JavaSource::default()));
    }
}
