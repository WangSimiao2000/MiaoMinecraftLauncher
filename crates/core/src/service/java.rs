use std::path::PathBuf;

use crate::error::Result;
use crate::java;

use super::LauncherService;

impl LauncherService {
    pub fn detect_java(&self) -> Vec<java::JavaInstallation> {
        java::detect_system_java()
    }

    pub async fn download_java_for_instance(
        &self,
        name: &str,
        progress_cb: Option<impl Fn(java::download::DownloadPhase) + Send + 'static>,
    ) -> Result<PathBuf> {
        let inst = self.load_instance(name)?;
        let meta = self.load_version_meta(&inst.minecraft_version)?;
        let required = meta.required_java_major();

        let installations = java::detect_system_java();
        if let Some(existing) = java::find_compatible_java(&installations, required) {
            return Ok(existing.path.clone());
        }

        let asset = java::download::fetch_latest_asset(&self.http, required).await?;
        let java_dir = self.config.data_dir.join("java");

        let java_bin = if let Some(cb) = progress_cb {
            java::download::download_and_extract_java_with_progress(
                &self.http, &asset, &java_dir, cb,
            )
            .await?
        } else {
            java::download::download_and_extract_java_with_progress(
                &self.http,
                &asset,
                &java_dir,
                |_| {},
            )
            .await?
        };

        Ok(java_bin)
    }
}
