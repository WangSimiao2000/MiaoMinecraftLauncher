use anyhow::Result;
use miao_core::config::LauncherConfig;
use miao_core::instance::Instance;
use miao_core::java;

pub fn cmd_java() {
    let installations = java::detect_system_java();

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

pub async fn cmd_download_java(config: &LauncherConfig, instance_name: &str) -> Result<()> {
    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    let inst = Instance::load_from(&instance_dir)?;

    let meta_path = config
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

    let http = reqwest::Client::new();
    let asset = miao_core::java::download::fetch_latest_asset(&http, required).await?;
    let total_mb = asset.binary.package.size as f64 / 1_000_000.0;
    println!("Downloading {} ({:.1} MB)...", asset.release_name, total_mb);

    let java_dir = config.data_dir.join("java");
    let java_bin = miao_core::java::download::download_and_extract_java_with_progress(
        &http,
        &asset,
        &java_dir,
        |phase| {
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
        },
    )
    .await?;

    println!("\n✓ Java {} installed at {}", required, java_bin.display());
    Ok(())
}
