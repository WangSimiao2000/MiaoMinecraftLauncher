use std::path::PathBuf;
use std::sync::Arc;

use egui::Context;
use tokio::sync::mpsc;

use crate::messages::AppEvent;

pub fn handle_download_java(
    required_major: u32,
    java_dir: PathBuf,
    launch_idx: usize,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let result = match miao_core::java::download::fetch_latest_asset(&http, required_major)
            .await
        {
            Ok(asset) => {
                let tx_progress = tx.clone();
                let ctx_progress = ctx.clone();
                miao_core::java::download::download_and_extract_java_with_progress(
                    &http,
                    &asset,
                    &java_dir,
                    move |phase| {
                        use miao_core::java::download::DownloadPhase;
                        let msg = match phase {
                            DownloadPhase::Downloading { downloaded, total } => {
                                format!(
                                    "Java {}: {:.1}/{:.1} MB",
                                    required_major,
                                    downloaded as f64 / 1_000_000.0,
                                    total as f64 / 1_000_000.0
                                )
                            }
                            DownloadPhase::Extracting => {
                                format!("Java {}: extracting...", required_major)
                            }
                        };
                        let _ = tx_progress.send(AppEvent::JavaProgress(msg));
                        ctx_progress.request_repaint();
                    },
                )
                .await
            }
            Err(e) => Err(e),
        };

        match result {
            Ok(_) => {
                let _ = tx.send(AppEvent::JavaInstalled { launch_idx });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::JavaFailed(format!(
                    "Java download failed: {}",
                    e
                )));
            }
        }
        ctx.request_repaint();
    })
}
