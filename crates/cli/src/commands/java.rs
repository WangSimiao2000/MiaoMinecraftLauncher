use anyhow::Result;
use miao_core::java;
use miao_core::service::LauncherService;

pub fn cmd_java(service: &LauncherService) {
    let installations = service.detect_java();

    if installations.is_empty() {
        println!("No Java installations found in standard paths.");
        println!("Searched: /usr/lib/jvm, /usr/local/lib/jvm, /usr/java");
        return;
    }

    println!("{:<8} {:<15} PATH", "MAJOR", "VERSION");
    println!("{}", "-".repeat(60));
    for j in &installations {
        println!(
            "{:<8} {:<15} {}",
            j.major_version,
            j.version,
            j.path.display()
        );
    }
}

pub async fn cmd_download_java(service: &LauncherService, instance_name: &str) -> Result<()> {
    let inst = service.load_instance(instance_name)?;

    let meta_path = service
        .config()
        .versions_dir()
        .join(&inst.minecraft_version)
        .join(format!("{}.json", &inst.minecraft_version));

    if !meta_path.exists() {
        anyhow::bail!("Version metadata not found for {}", inst.minecraft_version);
    }

    let meta_content = std::fs::read_to_string(&meta_path)?;
    let meta: miao_core::version::meta::VersionMeta = serde_json::from_str(&meta_content)?;
    let required = meta.required_java_major();

    let java_installations = java::detect_system_java();
    if java::find_compatible_java(&java_installations, required).is_some() {
        println!("✓ Java {} already available.", required);
        return Ok(());
    }

    println!(
        "Java {} required but not found. Downloading from Adoptium...",
        required
    );

    let java_bin = service
        .download_java_for_instance(instance_name, Some(|phase| {
            use miao_core::java::download::DownloadPhase;
            match phase {
                DownloadPhase::Downloading { downloaded, total } => {
                    eprint!(
                        "\r  {:.1}/{:.1} MB",
                        downloaded as f64 / 1_000_000.0,
                        total as f64 / 1_000_000.0
                    );
                }
                DownloadPhase::Extracting => {
                    eprintln!("\r  Extracting...          ");
                }
            }
        }))
        .await?;

    println!("\n✓ Java {} installed at {}", required, java_bin.display());
    Ok(())
}
