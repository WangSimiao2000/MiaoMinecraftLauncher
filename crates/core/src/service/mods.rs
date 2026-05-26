use crate::error::Result;
use crate::instance::Instance;
use crate::modrinth;

use super::LauncherService;

impl LauncherService {
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
}
