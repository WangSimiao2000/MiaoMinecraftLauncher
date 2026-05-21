use anyhow::Result;
use miao_core::config::LauncherConfig;
use miao_core::instance::Instance;

use super::{format_downloads, truncate};

pub fn cmd_mods(
    config: &LauncherConfig,
    instance_name: &str,
    toggle: Option<&str>,
    delete: Option<&str>,
) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let mods_dir = Instance::mods_dir(&instance_dir);
    let mut mods = miao_core::modmanager::scan_mods_dir(&mods_dir);

    if let Some(name) = toggle {
        let m = mods
            .iter_mut()
            .find(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("Mod '{}' not found", name))?;
        m.toggle()?;
        let state = if m.enabled { "enabled" } else { "disabled" };
        println!("✓ {} {}", name, state);
        return Ok(());
    }

    if let Some(name) = delete {
        let m = mods
            .iter()
            .find(|m| m.name == name)
            .ok_or_else(|| anyhow::anyhow!("Mod '{}' not found", name))?;
        m.delete()?;
        println!("✓ Deleted mod '{}'", name);
        return Ok(());
    }

    if mods.is_empty() {
        println!("No mods installed in '{}'.", instance_name);
    } else {
        println!("{:<6} NAME", "STATE");
        println!("{}", "-".repeat(40));
        for m in &mods {
            let state = if m.enabled { "  ✓" } else { "  ✗" };
            println!("{:<6} {}", state, m.name);
        }
    }
    Ok(())
}

pub async fn cmd_mod_search(
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> Result<()> {
    let result = miao_core::modrinth::api::search_mods(query, mc_version, loader, 15).await?;

    if result.hits.is_empty() {
        println!("No mods found for '{}'.", query);
        return Ok(());
    }

    println!("{:<30} {:<15} DOWNLOADS", "NAME", "ID");
    println!("{}", "-".repeat(60));
    for hit in &result.hits {
        println!(
            "{:<30} {:<15} {}",
            truncate(&hit.title, 28),
            hit.slug,
            format_downloads(hit.downloads)
        );
    }
    println!("\nInstall: miao mod-install <instance> <slug>");
    Ok(())
}

pub async fn cmd_mod_install(
    config: &LauncherConfig,
    instance_name: &str,
    project: &str,
) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let loader = inst
        .mod_loader
        .as_ref()
        .map(|l| l.loader_type.as_str())
        .unwrap_or("fabric");

    println!(
        "Installing '{}' (MC {}, {})...",
        project, inst.minecraft_version, loader
    );

    let mods_dir = Instance::mods_dir(&instance_dir);
    let results = miao_core::modrinth::api::install_mod_with_dependencies(
        project,
        &inst.minecraft_version,
        loader,
        &mods_dir,
    )
    .await?;

    if results.is_empty() {
        println!("No compatible version found for '{}'.", project);
    } else {
        for m in &results {
            let tag = if m.is_dependency { " (dep)" } else { "" };
            println!("  ✓ {}{}", m.filename, tag);
        }
        println!("Installed {} mod(s).", results.len());
    }
    Ok(())
}
