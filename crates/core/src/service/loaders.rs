use crate::error::Result;
use crate::instance::{Instance, ModLoaderConfig};
use crate::modloader::{self, LoaderFetchResult, ModLoaderType};

use super::LauncherService;

impl LauncherService {
    pub async fn fetch_loader_versions(&self, mc_version: &str) -> Result<LoaderFetchResult> {
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

    pub(crate) async fn resolve_latest_loader_version(
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
}
