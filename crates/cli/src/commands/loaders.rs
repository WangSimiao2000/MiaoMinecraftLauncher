use anyhow::Result;
use miao_core::config::LauncherConfig;
use miao_core::instance::Instance;
use miao_core::modloader::{self, ModLoaderType};

pub async fn cmd_loaders(_config: &LauncherConfig, mc_version: &str) -> Result<()> {
    let http = reqwest::Client::new();
    println!("Fetching loader compatibility for {}...", mc_version);

    let versions = modloader::fetch_all_loader_versions(&http, mc_version).await?;

    if versions.is_empty() {
        println!("No mod loaders available for {}.", mc_version);
        return Ok(());
    }

    for lt in &ModLoaderType::ALL {
        if let Some(loader_versions) = versions.get(lt) {
            let stable_count = loader_versions.iter().filter(|v| v.stable).count();
            let latest = &loader_versions[0].version;
            println!(
                "  {} — {} versions ({} stable), latest: {}",
                lt,
                loader_versions.len(),
                stable_count,
                latest
            );
        } else {
            println!("  {} — not available", lt);
        }
    }
    Ok(())
}

pub async fn cmd_upgrade_loader(
    config: &LauncherConfig,
    instance_name: &str,
    target_version: Option<&str>,
) -> Result<()> {
    use miao_core::modloader::{fabric, forge, neoforge, quilt};

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let mut inst = Instance::load_from(&instance_dir)?;
    let mc_version = inst.minecraft_version.clone();
    let http = reqwest::Client::new();

    let Some(current_loader) = &inst.mod_loader else {
        anyhow::bail!(
            "Instance '{}' has no mod loader installed. Use 'miao new' with --loader to create a modded instance.",
            instance_name
        );
    };

    let current_type = current_loader.loader_type.clone();
    let current_ver = current_loader.version.clone();

    let new_ver = match target_version {
        Some(v) => v.to_string(),
        None => match &current_type {
            ModLoaderType::Fabric => {
                let versions = fabric::fetch_loader_versions(&http, &mc_version).await?;
                versions
                    .iter()
                    .find(|v| v.loader.stable)
                    .or(versions.first())
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Fabric versions available"))?
            }
            ModLoaderType::Quilt => {
                let versions = quilt::fetch_loader_versions(&http, &mc_version).await?;
                versions
                    .first()
                    .map(|v| v.loader.version.clone())
                    .ok_or_else(|| anyhow::anyhow!("No Quilt versions available"))?
            }
            ModLoaderType::NeoForge => {
                let versions = neoforge::fetch_versions(&http, &mc_version).await?;
                versions
                    .first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No NeoForge versions available"))?
            }
            ModLoaderType::Forge => forge::fetch_recommended_version(&http, &mc_version)
                .await?
                .ok_or_else(|| anyhow::anyhow!("No Forge versions available"))?,
        },
    };

    println!(
        "Upgrading {} {} → {}...",
        current_type, current_ver, new_ver
    );
    let loader_config =
        modloader::install_loader(&http, &current_type, &mc_version, &new_ver, config).await?;
    inst.mod_loader = Some(loader_config);
    inst.save_to(&instance_dir)?;

    println!(
        "✓ {} upgraded to {} for '{}'",
        current_type, new_ver, instance_name
    );
    Ok(())
}
