use std::path::PathBuf;
use std::process::Command;

use crate::download::manager::ProgressCallback;
use crate::error::Result;
use crate::instance::Instance;
use crate::launch::{LaunchOptions, build_launch_command};
use crate::version::{assets, install};
use crate::{java, modrinth};

use super::LauncherService;

impl LauncherService {
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
        let native_tasks =
            install::collect_native_downloads(&meta, &self.config, &self.config.download_mirror);
        let mut all_tasks = tasks;
        all_tasks.extend(native_tasks);
        self.download_files(all_tasks, progress.clone()).await?;

        install::extract_natives(&meta, &self.config)?;

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

    pub async fn launch_instance(&mut self, name: &str) -> Result<Command> {
        let instance_dir = Instance::instance_dir(&self.config.instances_dir(), name);
        let inst = Instance::load_from(&instance_dir)?;

        let account = self.get_valid_account().await?;

        let meta = self.load_version_meta(&inst.minecraft_version)?;
        let required_java = meta.required_java_major();

        let report =
            crate::integrity::verify_instance_files(&inst.minecraft_version, &self.config)?;
        if !report.is_healthy() {
            let repair_tasks = report.repair_tasks();
            self.download_files(repair_tasks, None).await?;
        }

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
        modrinth::mrpack::import_mrpack(&self.http, mrpack_path, &self.config, name).await
    }
}
