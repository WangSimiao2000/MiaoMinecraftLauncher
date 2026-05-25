use anyhow::Result;
use miao_core::service::LauncherService;
use miao_core::version::VersionType;

pub fn cmd_list(service: &LauncherService) -> Result<()> {
    let instances = service.list_instances()?;

    if instances.is_empty() {
        println!("No instances found. Use 'miao new <version>' to create one.");
        return Ok(());
    }

    println!("{:<20} {:<12} MOD LOADER", "NAME", "MC VERSION");
    println!("{}", "-".repeat(50));
    for inst in &instances {
        let loader = inst
            .mod_loader
            .as_ref()
            .map(|l| format!("{} {}", l.loader_type, l.version))
            .unwrap_or_else(|| "Vanilla".to_string());
        println!(
            "{:<20} {:<12} {}",
            inst.name, inst.minecraft_version, loader
        );
    }

    Ok(())
}

pub async fn cmd_versions(service: &LauncherService, show_snapshots: bool) -> Result<()> {
    println!("Fetching version manifest...");
    let versions = service.fetch_versions().await?;

    let filtered: Vec<_> = if show_snapshots {
        versions.iter().take(30).collect()
    } else {
        versions
            .iter()
            .filter(|v| v.is_release())
            .take(20)
            .collect()
    };

    println!("{:<16} {:<10} RELEASE DATE", "VERSION", "TYPE");
    println!("{}", "-".repeat(50));
    for v in filtered {
        let type_str = match v.version_type {
            VersionType::Release => "release",
            VersionType::Snapshot => "snapshot",
            VersionType::OldBeta => "old_beta",
            VersionType::OldAlpha => "old_alpha",
        };
        println!("{:<16} {:<10} {}", v.id, type_str, v.release_time);
    }

    Ok(())
}

pub async fn cmd_new(
    service: &LauncherService,
    version: &str,
    name: Option<&str>,
    loader: Option<&str>,
    loader_version: Option<&str>,
) -> Result<()> {
    let instance_name = name.unwrap_or(version);
    println!("Creating instance '{}' (MC {})...", instance_name, version);

    let inst = service
        .create_instance(version, name, loader, loader_version, None)
        .await?;

    let loader_info = inst
        .mod_loader
        .as_ref()
        .map(|l| format!(" + {} {}", l.loader_type, l.version))
        .unwrap_or_default();

    println!(
        "✓ Instance '{}' created! (MC {}{})",
        inst.name, inst.minecraft_version, loader_info
    );
    println!("  Run: miao launch {}", inst.name);

    Ok(())
}

pub async fn cmd_launch(service: &LauncherService, instance_name: &str) -> Result<()> {
    println!("Launching {}...", instance_name);
    let mut cmd = service.launch_instance(instance_name).await?;
    let status = cmd.status()?;

    if !status.success() {
        anyhow::bail!("Minecraft exited with code: {:?}", status.code());
    }

    Ok(())
}

pub fn cmd_delete(service: &LauncherService, instance_name: &str) -> Result<()> {
    service.delete_instance(instance_name)?;
    println!("✓ Deleted instance '{}'", instance_name);
    Ok(())
}

pub fn cmd_account(service: &mut LauncherService, username: &str) -> Result<()> {
    service.add_offline_account(username)?;
    println!("✓ Added offline account '{}'", username);
    Ok(())
}

pub fn cmd_export(
    service: &LauncherService,
    instance_name: &str,
    output: Option<&str>,
) -> Result<()> {
    let output_path = output
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("Exporting '{}' as .mrpack...", instance_name);
    let result = service.export_instance(instance_name, &output_path)?;
    println!("✓ Exported to {}", result.display());
    Ok(())
}

pub async fn cmd_import(service: &LauncherService, path: &str, name: Option<&str>) -> Result<()> {
    let mrpack_path = std::path::Path::new(path);
    if !mrpack_path.exists() {
        anyhow::bail!("File not found: {}", path);
    }

    println!("Importing {}...", path);
    let inst = service.import_mrpack(mrpack_path, name).await?;

    let loader_info = inst
        .mod_loader
        .as_ref()
        .map(|l| format!(" + {} {}", l.loader_type, l.version))
        .unwrap_or_default();

    println!(
        "✓ Imported '{}' (MC {}{})",
        inst.name, inst.minecraft_version, loader_info
    );
    Ok(())
}

pub fn cmd_open(service: &LauncherService, instance_name: &str) -> Result<()> {
    let inst_dir = miao_core::instance::Instance::instance_dir(
        &service.config().instances_dir(),
        instance_name,
    );
    miao_core::instance::open_folder(&inst_dir)?;
    println!("Opened: {}", inst_dir.display());
    Ok(())
}
