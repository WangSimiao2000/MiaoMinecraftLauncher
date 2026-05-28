use std::path::PathBuf;
use std::sync::Arc;

use crate::error::Result;
use crate::java;

use super::LauncherService;

/// Streaming progress phase emitted while installing a Java runtime.
#[derive(Debug, Clone)]
pub enum JavaInstallPhase {
    /// Manifests being fetched; no byte progress yet.
    Preparing,
    /// File-level download in flight.
    Downloading {
        downloaded_files: usize,
        total_files: usize,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
}

impl LauncherService {
    pub fn detect_java(&self) -> Vec<java::JavaInstallation> {
        java::detect_system_java()
    }

    /// Download the Mojang JRE matching the Java requirement of the named instance and
    /// return the path to the resulting `java` binary.
    pub async fn download_java_for_instance(
        &self,
        name: &str,
        progress_cb: Option<impl Fn(JavaInstallPhase) + Send + Sync + 'static>,
    ) -> Result<PathBuf> {
        let inst = self.load_instance(name)?;
        let meta = self.load_version_meta(&inst.minecraft_version)?;
        let required = meta.required_java_major();

        let installations = java::detect_system_java();
        if let Some(existing) = java::find_compatible_java(&installations, required) {
            return Ok(existing.path.clone());
        }

        let cb_ref = progress_cb.as_ref();
        if let Some(cb) = cb_ref {
            cb(JavaInstallPhase::Preparing);
        }

        let plan = java::install::plan(&self.http, &self.config, required).await?;
        let total_bytes = plan.total_size;
        let total_files = plan.file_count;

        let progress: Option<crate::download::manager::ProgressCallback> = progress_cb.map(|cb| {
            let cb = Arc::new(cb);
            let cb_clone: crate::download::manager::ProgressCallback = Arc::new(move |p| {
                cb(JavaInstallPhase::Downloading {
                    downloaded_files: p.completed_files,
                    total_files,
                    downloaded_bytes: p.downloaded_bytes,
                    total_bytes,
                });
            });
            cb_clone
        });

        let java_bin = java::install::execute(plan, &self.config, progress).await?;
        Ok(java_bin)
    }
}
