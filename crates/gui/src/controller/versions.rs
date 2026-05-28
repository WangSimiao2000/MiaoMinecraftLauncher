use std::sync::Arc;

use egui::Context;
use miao_core::config::DownloadMirror;
use tokio::sync::mpsc;

use crate::messages::AppEvent;

pub fn handle_fetch_manifest(
    mirror: DownloadMirror,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        match miao_core::version::manifest::fetch_version_manifest(&*http, &mirror).await {
            Ok(all_versions) => {
                let releases: Vec<_> = all_versions
                    .iter()
                    .filter(|v| v.is_release())
                    .cloned()
                    .collect();
                let _ = tx.send(AppEvent::VersionsFetched {
                    all_versions,
                    releases,
                });
            }
            Err(_) => {
                let _ = tx.send(AppEvent::VersionsFetched {
                    all_versions: Vec::new(),
                    releases: Vec::new(),
                });
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_fetch_loader_versions(
    mc_version: String,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        match miao_core::modloader::fetch_all_loader_versions(&*http, &mc_version).await {
            Ok(result) => {
                let _ = tx.send(AppEvent::LoaderVersionsFetched {
                    versions: result.versions,
                    failed: result.failed,
                });
            }
            Err(_) => {
                let _ = tx.send(AppEvent::LoaderFetchFailed);
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_check_updates(tx: mpsc::UnboundedSender<AppEvent>, ctx: Context) {
    tokio::spawn(async move {
        let Ok(http) = reqwest::Client::builder()
            .user_agent(concat!("MMCL/", env!("CARGO_PKG_VERSION")))
            .build()
        else {
            return;
        };
        let url =
            "https://api.github.com/repos/WangSimiao2000/MiaoMinecraftLauncher/releases/latest";
        if let Ok(resp) = http.get(url).send().await
            && let Ok(json) = resp.json::<serde_json::Value>().await
            && let Some(tag) = json.get("tag_name").and_then(|v| v.as_str())
        {
            let current = env!("CARGO_PKG_VERSION");
            let remote = tag.trim_start_matches('v');
            if remote != current {
                let _ = tx.send(AppEvent::UpdateAvailable(tag.to_string()));
            }
        }
        ctx.request_repaint();
    });
}
