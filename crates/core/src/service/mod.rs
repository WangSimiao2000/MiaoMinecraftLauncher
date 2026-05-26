mod auth;
mod instance;
mod java;
mod loaders;
mod mods;

use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::download::manager::{DownloadManager, ProgressCallback};
use crate::error::Result;
use crate::version::meta::VersionMeta;
use crate::version::{VersionInfo, install, manifest};

pub struct LauncherService {
    pub(crate) config: LauncherConfig,
    pub(crate) http: reqwest::Client,
}

impl LauncherService {
    pub fn new(config: LauncherConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .user_agent(concat!("MiaoMinecraftLauncher/", env!("CARGO_PKG_VERSION")))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub fn config(&self) -> &LauncherConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut LauncherConfig {
        &mut self.config
    }

    pub fn update_config(&mut self, config: LauncherConfig) {
        self.config = config;
    }

    // ─── Version Operations ──────────────────────────────────────────────

    pub async fn fetch_versions(&self) -> Result<Vec<VersionInfo>> {
        manifest::fetch_version_manifest(&self.http, &self.config.download_mirror).await
    }

    pub async fn fetch_version_meta(&self, version_url: &str) -> Result<VersionMeta> {
        install::fetch_version_meta(&self.http, version_url, &self.config.download_mirror).await
    }

    // ─── Helpers ─────────────────────────────────────────────────────────

    pub(crate) async fn download_files(
        &self,
        tasks: Vec<DownloadTask>,
        progress: Option<ProgressCallback>,
    ) -> Result<()> {
        if tasks.is_empty() {
            return Ok(());
        }
        let dm = DownloadManager::new(
            self.config.download_mirror.clone(),
            self.config.max_concurrent_downloads,
        );
        let dm = if let Some(cb) = progress {
            dm.with_progress_callback(cb)
        } else {
            dm
        };
        dm.download_all(tasks).await
    }

    pub(crate) fn load_version_meta(&self, mc_version: &str) -> Result<VersionMeta> {
        let meta_path = self
            .config
            .versions_dir()
            .join(mc_version)
            .join(format!("{}.json", mc_version));

        if !meta_path.exists() {
            return Err(crate::error::MiaoError::Version(
                crate::error::VersionError::MetaNotFound {
                    version: mc_version.to_string(),
                },
            ));
        }

        let content = std::fs::read_to_string(&meta_path)?;
        let meta: VersionMeta = serde_json::from_str(&content)?;
        Ok(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(tmp: &std::path::Path) -> LauncherConfig {
        LauncherConfig {
            data_dir: tmp.to_path_buf(),
            config_file_override: Some(tmp.join("config.toml")),
            ..Default::default()
        }
    }

    #[test]
    fn new_service_holds_config() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config.clone());
        assert_eq!(service.config().data_dir, tmp.path());
    }

    #[test]
    fn config_mut_allows_modification() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let mut service = LauncherService::new(config);
        service.config_mut().max_concurrent_downloads = 16;
        assert_eq!(service.config().max_concurrent_downloads, 16);
    }

    #[test]
    fn update_config_replaces() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let mut service = LauncherService::new(config);

        let tmp2 = tempfile::tempdir().unwrap();
        let new_config = make_config(tmp2.path());
        service.update_config(new_config);
        assert_eq!(service.config().data_dir, tmp2.path());
    }

    #[test]
    fn active_account_none_by_default() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);
        assert!(service.active_account().is_none());
    }

    #[test]
    fn add_offline_account_and_retrieve() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = make_config(tmp.path());
        config.data_dir = tmp.path().to_path_buf();
        let mut service = LauncherService::new(config);
        service.add_offline_account("TestPlayer").unwrap();

        let account = service.active_account().unwrap();
        assert_eq!(account.username(), "TestPlayer");
        assert!(account.is_offline());
    }

    #[test]
    fn list_instances_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);
        let instances = service.list_instances().unwrap();
        assert!(instances.is_empty());
    }

    #[test]
    fn list_and_load_instance() {
        use crate::instance::Instance;
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);

        let inst_dir = Instance::instance_dir(&tmp.path().join("instances"), "my-server");
        std::fs::create_dir_all(&inst_dir).unwrap();
        let inst = Instance::new("my-server", "1.20.4");
        inst.save_to(&inst_dir).unwrap();

        let instances = service.list_instances().unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].name, "my-server");

        let loaded = service.load_instance("my-server").unwrap();
        assert_eq!(loaded.minecraft_version, "1.20.4");
    }

    #[test]
    fn delete_instance_removes_dir() {
        use crate::instance::Instance;
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);

        let inst_dir = Instance::instance_dir(&tmp.path().join("instances"), "to-delete");
        std::fs::create_dir_all(&inst_dir).unwrap();
        let inst = Instance::new("to-delete", "1.20.4");
        inst.save_to(&inst_dir).unwrap();

        assert!(inst_dir.exists());
        service.delete_instance("to-delete").unwrap();
        assert!(!inst_dir.exists());
    }

    #[test]
    fn detect_java_returns_vec() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);
        let _ = service.detect_java();
    }

    #[test]
    fn load_version_meta_missing_returns_error() {
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);
        let result = service.load_version_meta("1.99.99");
        assert!(result.is_err());
    }

    #[test]
    fn export_instance_roundtrip() {
        use crate::instance::Instance;
        let tmp = tempfile::tempdir().unwrap();
        let config = make_config(tmp.path());
        let service = LauncherService::new(config);

        let inst_dir = Instance::instance_dir(&tmp.path().join("instances"), "export-test");
        std::fs::create_dir_all(&inst_dir).unwrap();
        Instance::create_directories(&inst_dir).unwrap();
        let inst = Instance::new("export-test", "1.20.4");
        inst.save_to(&inst_dir).unwrap();

        let output_dir = tmp.path().join("exports");
        std::fs::create_dir_all(&output_dir).unwrap();

        let mrpack_path = service.export_instance("export-test", &output_dir).unwrap();
        assert!(mrpack_path.exists());
        assert!(mrpack_path.to_str().unwrap().ends_with(".mrpack"));
    }
}
