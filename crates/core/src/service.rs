use std::path::PathBuf;
use std::process::Command;

use crate::error::Result;

use crate::auth::AuthMethod;
use crate::auth::offline::create_offline_account;
use crate::config::LauncherConfig;
use crate::download::DownloadTask;
use crate::download::manager::{DownloadManager, ProgressCallback};
use crate::instance::{Instance, ModLoaderConfig};
use crate::java;
use crate::launch::{LaunchOptions, build_launch_command};
use crate::modloader::{self, ModLoaderType};
use crate::modrinth;
use crate::version::{VersionInfo, assets, install, manifest, meta::VersionMeta};

pub struct LauncherService {
    config: LauncherConfig,
    http: reqwest::Client,
}

impl LauncherService {
    pub fn new(config: LauncherConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .user_agent("MiaoMinecraftLauncher/0.1.0")
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

    // ─── Instance Operations ─────────────────────────────────────────────

    pub fn list_instances(&self) -> Result<Vec<Instance>> {
        crate::instance::list_instances(&self.config.instances_dir())
    }

    pub fn load_instance(&self, name: &str) -> Result<Instance> {
        let dir = Instance::instance_dir(&self.config.instances_dir(), name);
        Instance::load_from(&dir)
    }

    pub fn delete_instance(&self, name: &str) -> Result<()> {
        crate::instance::delete_instance(&self.config.instances_dir(), name)
    }

    pub async fn create_instance(
        &self,
        mc_version: &str,
        name: Option<&str>,
        loader: Option<&str>,
        loader_version: Option<&str>,
        progress: Option<ProgressCallback>,
    ) -> Result<Instance> {
        let instance_name = name.unwrap_or(mc_version);

        let versions = self.fetch_versions().await?;
        let version_info = versions
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or_else(|| {
                crate::error::MiaoError::Other(format!("Version '{}' not found", mc_version))
            })?;

        let meta = self.fetch_version_meta(&version_info.url).await?;
        install::save_version_meta(&meta, &self.config)?;

        let tasks = install::all_download_tasks(&meta, &self.config, &self.config.download_mirror);
        self.download_files(tasks, progress.clone()).await?;

        let asset_index_task = install::collect_asset_index_download(
            &meta,
            &self.config,
            &self.config.download_mirror,
        );
        let asset_index_path = asset_index_task.dest.clone();
        self.download_files(vec![asset_index_task], progress.clone())
            .await?;

        if asset_index_path.exists() {
            let asset_index = assets::fetch_asset_index(&asset_index_path).await?;
            let asset_tasks = assets::collect_asset_downloads(
                &asset_index,
                &self.config,
                &self.config.download_mirror,
            );
            self.download_files(asset_tasks, progress).await?;
        }

        let mut inst = Instance::new(instance_name, mc_version);

        if let Some(loader_str) = loader {
            let loader_config = self
                .install_loader(mc_version, loader_str, loader_version)
                .await?;
            inst.mod_loader = Some(loader_config);
        }

        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), instance_name);
        inst.save_to(&instance_dir)?;
        Instance::create_directories(&instance_dir)?;

        Ok(inst)
    }

    // ─── Launch ──────────────────────────────────────────────────────────

    pub async fn launch_instance(&mut self, name: &str) -> Result<Command> {
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), name);
        let inst = Instance::load_from(&instance_dir)?;

        let account = self.get_valid_account().await?;

        let meta = self.load_version_meta(&inst.minecraft_version)?;
        let required_java = meta.required_java_major();

        let java_path = inst
            .java_path
            .clone()
            .or_else(|| {
                let installations = java::detect_system_java();
                java::find_compatible_java(&installations, required_java).map(|j| j.path.clone())
            })
            .ok_or_else(|| {
                crate::error::MiaoError::Java(crate::error::JavaError::NotFound {
                    required_major: required_java,
                })
            })?;

        let options = LaunchOptions {
            game_dir: instance_dir,
            java_path,
            version_meta: meta,
            instance: inst,
            auth: account,
            config: self.config.clone(),
        };

        build_launch_command(&options)
    }

    /// Returns a valid account, refreshing the Microsoft token if expired.
    /// Falls back to offline account if no accounts are configured.
    pub async fn get_valid_account(&mut self) -> Result<AuthMethod> {
        use crate::auth::microsoft::MicrosoftAuth;
        use crate::auth::MS_CLIENT_ID;

        let idx = self.config.active_account_index.unwrap_or(0);
        let account = self
            .config
            .accounts
            .get(idx)
            .cloned()
            .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

        match account {
            AuthMethod::Microsoft(ref ms_acc) if ms_acc.is_expired() => {
                let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());
                match auth.refresh(ms_acc).await {
                    Ok(refreshed) => {
                        let new_auth = AuthMethod::Microsoft(refreshed);
                        if let Some(stored) = self.config.accounts.get_mut(idx) {
                            *stored = new_auth.clone();
                        }
                        let _ = self.config.save();
                        Ok(new_auth)
                    }
                    Err(e) => Err(crate::error::MiaoError::Auth(
                        crate::error::AuthError::RefreshFailed(format!(
                            "Failed to refresh token for '{}': {}",
                            ms_acc.username, e
                        )),
                    )),
                }
            }
            _ => Ok(account),
        }
    }

    // ─── Java ────────────────────────────────────────────────────────────

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

    // ─── Mod Loaders ─────────────────────────────────────────────────────

    pub async fn fetch_loader_versions(
        &self,
        mc_version: &str,
    ) -> Result<std::collections::HashMap<ModLoaderType, Vec<modloader::ModLoaderVersion>>> {
        modloader::fetch_all_loader_versions(&self.http, mc_version).await
    }

    pub async fn install_loader(
        &self,
        mc_version: &str,
        loader_str: &str,
        loader_version: Option<&str>,
    ) -> Result<ModLoaderConfig> {
        let lt = ModLoaderType::parse(loader_str)?;
        let ver = match loader_version {
            Some(v) => v.to_string(),
            None => self.resolve_latest_loader_version(&lt, mc_version).await?,
        };
        modloader::install_loader(&self.http, &lt, mc_version, &ver, &self.config).await
    }

    pub async fn upgrade_loader(
        &self,
        instance_name: &str,
        target_version: Option<&str>,
    ) -> Result<Instance> {
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), instance_name);
        let mut inst = Instance::load_from(&instance_dir)?;
        let mc_version = inst.minecraft_version.clone();

        let current_loader = inst.mod_loader.as_ref().ok_or_else(|| {
            crate::error::MiaoError::Other(format!(
                "Instance '{}' has no mod loader installed",
                instance_name
            ))
        })?;

        let current_type = current_loader.loader_type.clone();

        let new_ver = match target_version {
            Some(v) => v.to_string(),
            None => {
                self.resolve_latest_loader_version(&current_type, &mc_version)
                    .await?
            }
        };

        let loader_config = modloader::install_loader(
            &self.http,
            &current_type,
            &mc_version,
            &new_ver,
            &self.config,
        )
        .await?;
        inst.mod_loader = Some(loader_config);
        inst.save_to(&instance_dir)?;

        Ok(inst)
    }

    async fn resolve_latest_loader_version(
        &self,
        loader_type: &ModLoaderType,
        mc_version: &str,
    ) -> Result<String> {
        use crate::modloader::{fabric, forge, neoforge, quilt};

        let version = match loader_type {
            ModLoaderType::Fabric => {
                let versions = fabric::fetch_loader_versions(&self.http, mc_version).await?;
                versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| {
                        crate::error::MiaoError::Other(format!(
                            "No Fabric versions for MC {}",
                            mc_version
                        ))
                    })?
            }
            ModLoaderType::Quilt => {
                let versions = quilt::fetch_loader_versions(&self.http, mc_version).await?;
                versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| {
                        crate::error::MiaoError::Other(format!(
                            "No Quilt versions for MC {}",
                            mc_version
                        ))
                    })?
            }
            ModLoaderType::NeoForge => {
                let versions = neoforge::fetch_versions(&self.http, mc_version).await?;
                versions.first().cloned().ok_or_else(|| {
                    crate::error::MiaoError::Other(format!(
                        "No NeoForge versions for MC {}",
                        mc_version
                    ))
                })?
            }
            ModLoaderType::Forge => forge::fetch_recommended_version(&self.http, mc_version)
                .await?
                .ok_or_else(|| {
                    crate::error::MiaoError::Other(format!(
                        "No Forge versions for MC {}",
                        mc_version
                    ))
                })?,
        };

        Ok(version)
    }

    // ─── Modrinth Mods ───────────────────────────────────────────────────

    pub async fn search_mods(
        &self,
        query: &str,
        mc_version: Option<&str>,
        loader: Option<&str>,
        limit: u32,
    ) -> Result<modrinth::api::SearchResult> {
        modrinth::api::search_mods(&self.http, query, mc_version, loader, limit).await
    }

    pub async fn install_mod(
        &self,
        instance_name: &str,
        project_id: &str,
    ) -> Result<Vec<modrinth::api::InstalledMod>> {
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), instance_name);
        let inst = Instance::load_from(&instance_dir)?;

        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| l.loader_type.as_str())
            .unwrap_or("fabric");

        let mods_dir = Instance::mods_dir(&instance_dir);
        modrinth::api::install_mod_with_dependencies(
            &self.http,
            project_id,
            &inst.minecraft_version,
            loader,
            &mods_dir,
        )
        .await
    }

    // ─── Mrpack Import/Export ─────────────────────────────────────────────

    pub fn export_instance(
        &self,
        instance_name: &str,
        output_dir: &std::path::Path,
    ) -> Result<PathBuf> {
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), instance_name);
        let inst = Instance::load_from(&instance_dir)?;
        modrinth::mrpack::export_mrpack(&instance_dir, &inst, output_dir)
    }

    pub async fn import_mrpack(
        &self,
        mrpack_path: &std::path::Path,
        name: Option<&str>,
    ) -> Result<Instance> {
        modrinth::mrpack::import_mrpack(mrpack_path, &self.config, name).await
    }

    // ─── Helpers ─────────────────────────────────────────────────────────

    async fn download_files(
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

    fn load_version_meta(&self, mc_version: &str) -> Result<VersionMeta> {
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

impl LauncherService {
    pub fn add_offline_account(&mut self, username: &str) -> Result<()> {
        let account = create_offline_account(username);
        self.config.accounts.push(AuthMethod::Offline(account));
        if self.config.active_account_index.is_none() {
            self.config.active_account_index = Some(0);
        }
        self.config.save()?;
        Ok(())
    }

    pub fn active_account(&self) -> Option<&AuthMethod> {
        self.config
            .active_account_index
            .and_then(|idx| self.config.accounts.get(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(tmp: &std::path::Path) -> LauncherConfig {
        LauncherConfig {
            data_dir: tmp.to_path_buf(),
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
