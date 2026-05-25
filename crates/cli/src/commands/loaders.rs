use anyhow::Result;
use miao_core::modloader::ModLoaderType;
use miao_core::service::LauncherService;

pub async fn cmd_loaders(service: &LauncherService, mc_version: &str) -> Result<()> {
    println!("Fetching loader compatibility for {}...", mc_version);

    let versions = service.fetch_loader_versions(mc_version).await?;

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
    service: &LauncherService,
    instance_name: &str,
    target_version: Option<&str>,
) -> Result<()> {
    let inst = service.load_instance(instance_name)?;
    let current_loader = inst.mod_loader.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Instance '{}' has no mod loader installed.", instance_name)
    })?;

    let current_type = &current_loader.loader_type;
    let current_ver = &current_loader.version;

    println!("Upgrading {} for '{}'...", current_type, instance_name);

    let updated = service
        .upgrade_loader(instance_name, target_version)
        .await?;

    let new_ver = updated
        .mod_loader
        .as_ref()
        .map(|l| l.version.as_str())
        .unwrap_or("?");

    println!(
        "✓ {} upgraded {} → {} for '{}'",
        current_type, current_ver, new_ver, instance_name
    );
    Ok(())
}
