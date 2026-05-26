use std::sync::Arc;

use egui::Context;
use miao_core::config::LauncherConfig;
use miao_core::download::manager::DownloadManager;
use miao_core::instance::Instance;
use miao_core::version::VersionInfo;
use tokio::sync::mpsc;

use crate::messages::AppEvent;

#[allow(clippy::too_many_arguments)]
pub fn handle_create_instance(
    task_id: String,
    ver: VersionInfo,
    name: String,
    loader: Option<(String, String)>,
    config: LauncherConfig,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let _ = tx.send(AppEvent::InstallStatus(format!("Creating '{}'...", name)));
        ctx.request_repaint();

        let result = do_create_instance(
            &task_id,
            &ver,
            &name,
            loader.as_ref().map(|(lt, lv)| (lt.as_str(), lv.as_str())),
            &config,
            &http,
            tx.clone(),
            ctx.clone(),
        )
        .await;

        match result {
            Ok(_) => {
                let _ = tx.send(AppEvent::InstallFinished {
                    task_id,
                    success: true,
                    message: format!("✓ '{}' created!", name),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::InstallFinished {
                    task_id,
                    success: false,
                    message: format!("✗ Failed: {}", e),
                });
            }
        }
        ctx.request_repaint();
    })
}

pub fn handle_launch_instance(
    idx: usize,
    instance: Instance,
    config: LauncherConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    std::thread::spawn(move || {
        stream_game_process(idx, instance, config, tx, ctx);
    });
}

pub fn handle_export_instance(
    instance: Instance,
    config: LauncherConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    std::thread::spawn(move || {
        let folder = rfd::FileDialog::new()
            .set_title("Export .mrpack")
            .pick_folder();

        let Some(output_path) = folder else {
            let _ = tx.send(AppEvent::ExportResult("Export cancelled.".to_string()));
            ctx.request_repaint();
            return;
        };

        let _ = tx.send(AppEvent::InstallStatus(format!(
            "Exporting '{}'...",
            instance.name
        )));
        ctx.request_repaint();

        let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance.name);
        match miao_core::modrinth::mrpack::export_mrpack(&instance_dir, &instance, &output_path) {
            Ok(path) => {
                let _ = tx.send(AppEvent::ExportResult(format!(
                    "✓ Exported to {}",
                    path.display()
                )));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ExportResult(format!("✗ Export failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_import_mrpack(
    task_id: String,
    config: LauncherConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    std::thread::spawn(move || {
        let file = rfd::FileDialog::new()
            .set_title("Import .mrpack")
            .add_filter("Modrinth Modpack", &["mrpack"])
            .pick_file();

        let Some(mrpack_path) = file else {
            let _ = tx.send(AppEvent::ImportResult {
                success: false,
                message: "Import cancelled.".to_string(),
            });
            let _ = tx.send(AppEvent::InstallFinished {
                task_id,
                success: false,
                message: "Import cancelled.".to_string(),
            });
            ctx.request_repaint();
            return;
        };

        let _ = tx.send(AppEvent::InstallStatus(format!(
            "Importing {}...",
            mrpack_path.display()
        )));
        ctx.request_repaint();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            match miao_core::modrinth::mrpack::import_mrpack(&mrpack_path, &config, None).await {
                Ok(inst) => {
                    let loader_info = inst
                        .mod_loader
                        .as_ref()
                        .map(|l| format!(" + {} {}", l.loader_type, l.version))
                        .unwrap_or_default();
                    let msg = format!(
                        "✓ Imported '{}' (MC {}{})",
                        inst.name, inst.minecraft_version, loader_info
                    );
                    let _ = tx.send(AppEvent::ImportResult {
                        success: true,
                        message: msg.clone(),
                    });
                    let _ = tx.send(AppEvent::InstallFinished {
                        task_id,
                        success: true,
                        message: msg,
                    });
                }
                Err(e) => {
                    let msg = format!("✗ Import failed: {}", e);
                    let _ = tx.send(AppEvent::ImportResult {
                        success: false,
                        message: msg.clone(),
                    });
                    let _ = tx.send(AppEvent::InstallFinished {
                        task_id,
                        success: false,
                        message: msg,
                    });
                }
            }
            ctx.request_repaint();
        });
    });
}

fn stream_game_process(
    _idx: usize,
    instance: Instance,
    mut config: LauncherConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    use miao_core::auth::AuthMethod;
    use miao_core::auth::microsoft::MicrosoftAuth;
    use miao_core::auth::offline::create_offline_account;
    use miao_core::auth::MS_CLIENT_ID;
    use miao_core::java;
    use miao_core::launch::{LaunchOptions, build_launch_command};
    use std::io::BufRead;

    let idx = config.active_account_index.unwrap_or(0);
    let account = config
        .accounts
        .get(idx)
        .cloned()
        .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

    let account = match &account {
        AuthMethod::Microsoft(ms_acc) if ms_acc.is_expired() => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());
            match rt.block_on(auth.refresh(ms_acc)) {
                Ok(refreshed) => {
                    let new_auth = AuthMethod::Microsoft(refreshed);
                    if let Some(stored) = config.accounts.get_mut(idx) {
                        *stored = new_auth.clone();
                    }
                    let _ = config.save();
                    new_auth
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!(
                        "Token refresh failed for '{}': {}. Please re-login.",
                        ms_acc.username, e
                    )));
                    ctx.request_repaint();
                    return;
                }
            }
        }
        _ => account,
    };

    let java_installations = java::detect_java_with_data_dir(&config.data_dir);
    let meta_path = config
        .versions_dir()
        .join(&instance.minecraft_version)
        .join(format!("{}.json", instance.minecraft_version));

    if !meta_path.exists() {
        let _ = tx.send(AppEvent::Error(format!(
            "Version meta not found for {}.",
            instance.minecraft_version
        )));
        ctx.request_repaint();
        return;
    }

    let meta_content = match std::fs::read_to_string(&meta_path) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Error: {}", e)));
            ctx.request_repaint();
            return;
        }
    };

    let meta: miao_core::version::meta::VersionMeta = match serde_json::from_str(&meta_content) {
        Ok(m) => m,
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Parse error: {}", e)));
            ctx.request_repaint();
            return;
        }
    };

    let required_java = meta.required_java_major();
    let java_path = instance.java_path.clone().or_else(|| {
        java::find_compatible_java(&java_installations, required_java).map(|j| j.path.clone())
    });

    let Some(java_path) = java_path else {
        let _ = tx.send(AppEvent::Error(format!(
            "Java {} not found — trigger download dialog",
            required_java
        )));
        ctx.request_repaint();
        return;
    };

    let instance_dir = Instance::instance_dir(&config.instances_dir(), &instance.name);
    let options = LaunchOptions {
        game_dir: instance_dir,
        java_path,
        version_meta: meta,
        instance: instance.clone(),
        auth: account,
        config: config.clone(),
    };

    match build_launch_command(&options) {
        Ok(mut cmd) => {
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            match cmd.spawn() {
                Ok(mut child) => {
                    let _ = tx.send(AppEvent::InstallStatus(format!(
                        "Launched {}",
                        instance.name
                    )));
                    ctx.request_repaint();

                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    let tx2 = tx.clone();
                    let ctx2 = ctx.clone();
                    let stdout_handle = stdout.map(|out| {
                        let tx = tx.clone();
                        let ctx = ctx.clone();
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(out);
                            for line in reader.lines() {
                                let Ok(line) = line else { break };
                                let _ = tx.send(AppEvent::GameLogLine(line));
                                ctx.request_repaint();
                            }
                        })
                    });

                    let stderr_handle = stderr.map(|err| {
                        let tx = tx2;
                        let ctx = ctx2;
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(err);
                            for line in reader.lines() {
                                let Ok(line) = line else { break };
                                let _ = tx.send(AppEvent::GameLogLine(format!("[ERR] {}", line)));
                                ctx.request_repaint();
                            }
                        })
                    });

                    if let Some(h) = stdout_handle {
                        let _ = h.join();
                    }
                    if let Some(h) = stderr_handle {
                        let _ = h.join();
                    }
                    let _ = child.wait();
                    let _ = tx.send(AppEvent::GameExited);
                    ctx.request_repaint();
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(format!("Launch failed: {}", e)));
                    ctx.request_repaint();
                }
            }
        }
        Err(e) => {
            let _ = tx.send(AppEvent::Error(format!("Command error: {}", e)));
            ctx.request_repaint();
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn do_create_instance(
    task_id: &str,
    ver: &VersionInfo,
    instance_name: &str,
    loader: Option<(&str, &str)>,
    config: &LauncherConfig,
    http: &reqwest::Client,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) -> anyhow::Result<()> {
    use miao_core::modloader::ModLoaderType;

    let meta =
        miao_core::version::install::fetch_version_meta(http, &ver.url, &config.download_mirror)
            .await?;
    miao_core::version::install::save_version_meta(&meta, config)?;

    let tasks =
        miao_core::version::install::all_download_tasks(&meta, config, &config.download_mirror);

    let _ = tx.send(AppEvent::InstallProgress {
        task_id: task_id.to_string(),
        completed: 0,
        total: tasks.len(),
        label: "Downloading libraries".to_string(),
    });
    ctx.request_repaint();

    let tx_cb = tx.clone();
    let ctx_cb = ctx.clone();
    let task_id_cb = task_id.to_string();
    let dm = DownloadManager::new(
        config.download_mirror.clone(),
        config.max_concurrent_downloads,
    )
    .with_progress_callback(Arc::new(move |p| {
        let _ = tx_cb.send(AppEvent::InstallProgress {
            task_id: task_id_cb.clone(),
            completed: p.completed_files,
            total: p.total_files,
            label: "Downloading libraries".to_string(),
        });
        ctx_cb.request_repaint();
    }));
    dm.download_all(tasks).await?;

    let asset_index_path = config
        .assets_dir()
        .join("indexes")
        .join(format!("{}.json", meta.asset_index.id));

    if asset_index_path.exists() {
        let asset_index = miao_core::version::assets::fetch_asset_index(&asset_index_path).await?;
        let asset_tasks = miao_core::version::assets::collect_asset_downloads(
            &asset_index,
            config,
            &config.download_mirror,
        );

        let _ = tx.send(AppEvent::InstallProgress {
            task_id: task_id.to_string(),
            completed: 0,
            total: asset_tasks.len(),
            label: "Downloading assets".to_string(),
        });
        ctx.request_repaint();

        let tx_cb2 = tx.clone();
        let ctx_cb2 = ctx.clone();
        let task_id_cb2 = task_id.to_string();
        let dm2 = DownloadManager::new(
            config.download_mirror.clone(),
            config.max_concurrent_downloads,
        )
        .with_progress_callback(Arc::new(move |p| {
            let _ = tx_cb2.send(AppEvent::InstallProgress {
                task_id: task_id_cb2.clone(),
                completed: p.completed_files,
                total: p.total_files,
                label: "Downloading assets".to_string(),
            });
            ctx_cb2.request_repaint();
        }));
        dm2.download_all(asset_tasks).await?;
    }

    let mut inst = Instance::new(instance_name, &ver.id);

    if let Some((loader_type_str, loader_version)) = loader {
        let lt = ModLoaderType::ALL
            .iter()
            .find(|t| t.as_str() == loader_type_str)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Unknown loader: {}", loader_type_str))?;

        let loader_config =
            miao_core::modloader::install_loader(http, &lt, &ver.id, loader_version, config)
                .await?;
        inst.mod_loader = Some(loader_config);
    }

    let instance_dir = Instance::instance_dir(&config.instances_dir(), instance_name);
    inst.save_to(&instance_dir)?;
    Instance::create_directories(&instance_dir)?;

    Ok(())
}
