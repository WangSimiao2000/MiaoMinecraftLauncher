use std::path::PathBuf;
use std::sync::Arc;

use egui::Context;
use miao_core::config::LauncherConfig;
use tokio::sync::mpsc;

use crate::messages::AppEvent;

/// Spawn a background Java detection scan. If `force` is false and a fresh cache exists
/// the result is delivered immediately without re-scanning.
pub fn handle_refresh_java(
    data_dir: PathBuf,
    force: bool,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    if !force && let Some(cached) = miao_core::java::cached_javas_for(&data_dir) {
        let _ = tx.send(AppEvent::JavaDetected {
            installations: cached,
        });
        ctx.request_repaint();
        return;
    }

    tokio::spawn(async move {
        if force {
            miao_core::java::invalidate_java_cache();
        }
        let installations = tokio::task::spawn_blocking(move || {
            miao_core::java::detect_java_with_data_dir(&data_dir)
        })
        .await
        .unwrap_or_default();
        let _ = tx.send(AppEvent::JavaDetected { installations });
        ctx.request_repaint();
    });
}

pub fn handle_download_java(
    task_id: String,
    required_major: u32,
    launch_idx: usize,
    config: LauncherConfig,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let label_prefix = format!("Java {}", required_major);

        // Initial 0% so the progress bar renders immediately while we fetch manifests.
        let _ = tx.send(AppEvent::InstallProgress {
            task_id: task_id.clone(),
            completed: 0,
            total: 100,
            label: format!("{}: fetching manifest...", label_prefix),
        });
        ctx.request_repaint();

        // Phase 1: plan (source-specific manifest fetches).
        let plan = match miao_core::java::install::plan(&http, &config, required_major).await {
            Ok(plan) => plan,
            Err(e) => {
                emit_failure(&tx, task_id, format!("{}", e));
                ctx.request_repaint();
                return;
            }
        };

        let total_size_mb = plan.total_size as f64 / 1_000_000.0;
        let variant = plan.variant.clone();
        let actual_major = plan.major;
        let source_name = plan.source_name;
        let header = if actual_major == required_major {
            format!("{} ({}/{})", label_prefix, source_name, variant)
        } else {
            // Source upgraded the request (e.g. 11 → 16, 18 → 21).
            format!(
                "{} → Java {} ({}/{})",
                label_prefix, actual_major, source_name, variant
            )
        };
        let _ = tx.send(AppEvent::InstallProgress {
            task_id: task_id.clone(),
            completed: 1,
            total: 100,
            label: format!(
                "{}: downloading {} files, {:.1} MB",
                header, plan.file_count, total_size_mb
            ),
        });
        ctx.request_repaint();

        // Phase 2: download. The DownloadManager streams a progress callback per file
        // completion, which we translate to a 1-99% range in the install progress bar.
        let total_files = plan.file_count.max(1);
        let total_size = plan.total_size.max(1);
        let label_template = header;
        let tx_progress = tx.clone();
        let ctx_progress = ctx.clone();
        let task_id_progress = task_id.clone();

        let progress_cb: miao_core::download::manager::ProgressCallback = Arc::new(move |p| {
            // Map bytes downloaded → 1..99 so phase 1 (1%) and phase 3 (100%) are
            // distinguishable.
            let pct = 1 + (p.downloaded_bytes as f64 / total_size as f64 * 98.0) as usize;
            let _ = tx_progress.send(AppEvent::InstallProgress {
                task_id: task_id_progress.clone(),
                completed: pct.min(99),
                total: 100,
                label: format!(
                    "{}: {}/{} files, {:.1}/{:.1} MB",
                    label_template,
                    p.completed_files,
                    total_files,
                    p.downloaded_bytes as f64 / 1_000_000.0,
                    total_size as f64 / 1_000_000.0
                ),
            });
            ctx_progress.request_repaint();
        });

        let result = miao_core::java::install::execute(plan, &config, Some(progress_cb)).await;

        match result {
            Ok(_) => {
                // Invalidate detection cache so the new install is found on next probe.
                miao_core::java::invalidate_java_cache();
                let _ = tx.send(AppEvent::InstallProgress {
                    task_id: task_id.clone(),
                    completed: 100,
                    total: 100,
                    label: format!("{}: completed", label_prefix),
                });
                let _ = tx.send(AppEvent::JavaInstalled { launch_idx });
                let _ = tx.send(AppEvent::InstallFinished {
                    task_id,
                    success: true,
                    message: format!("{} installed", label_prefix),
                });
            }
            Err(e) => emit_failure(&tx, task_id, format!("{}", e)),
        }
        ctx.request_repaint();
    })
}

fn emit_failure(tx: &mpsc::UnboundedSender<AppEvent>, task_id: String, message: String) {
    let _ = tx.send(AppEvent::JavaFailed(format!(
        "Java download failed: {}",
        message
    )));
    let _ = tx.send(AppEvent::InstallFinished {
        task_id,
        success: false,
        message: format!("Java download failed: {}", message),
    });
}
