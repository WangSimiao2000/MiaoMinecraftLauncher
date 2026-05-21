use anyhow::Result;
use miao_core::config::LauncherConfig;
use miao_core::instance::{self, Instance};

pub fn cmd_resources(
    config: &LauncherConfig,
    instance_name: &str,
    delete: Option<&str>,
) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let dir = Instance::resourcepacks_dir(&instance_dir);
    let packs = miao_core::resource::scan_resourcepacks(&dir);

    if let Some(name) = delete {
        let p = packs
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow::anyhow!("Resource pack '{}' not found", name))?;
        p.delete()?;
        println!("✓ Deleted resource pack '{}'", name);
        return Ok(());
    }

    if packs.is_empty() {
        println!("No resource packs in '{}'.", instance_name);
    } else {
        for p in &packs {
            println!("  {}", p.name);
        }
    }
    Ok(())
}

pub fn cmd_shaders(
    config: &LauncherConfig,
    instance_name: &str,
    delete: Option<&str>,
) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let dir = Instance::shaderpacks_dir(&instance_dir);
    let shaders = miao_core::resource::scan_shaderpacks(&dir);

    if let Some(name) = delete {
        let s = shaders
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Shader pack '{}' not found", name))?;
        s.delete()?;
        println!("✓ Deleted shader pack '{}'", name);
        return Ok(());
    }

    if shaders.is_empty() {
        println!("No shader packs in '{}'.", instance_name);
    } else {
        for s in &shaders {
            println!("  {}", s.name);
        }
    }
    Ok(())
}

pub fn cmd_saves(config: &LauncherConfig, instance_name: &str, delete: Option<&str>) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let saves = instance::list_saves(&instance_dir);

    if let Some(name) = delete {
        let s = saves
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("World '{}' not found", name))?;
        s.delete()?;
        println!("✓ Deleted world '{}'", name);
        return Ok(());
    }

    if saves.is_empty() {
        println!("No worlds in '{}'.", instance_name);
    } else {
        for s in &saves {
            println!("  {}", s.name);
        }
    }
    Ok(())
}
