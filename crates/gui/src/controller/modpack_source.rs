use std::sync::Arc;
use std::time::Duration;

use egui::Context;
use miao_core::config::LauncherConfig;
use miao_core::http::ReqwestClient;
use miao_core::modpack_source::{
    InstallPhase, InstallPlan, InstallProgressSink, LiveInstallExecutor, LiveResolverDataSource,
    install, parse_manifest, parse_pack, resolve,
};
use tokio::sync::mpsc;

use crate::messages::AppEvent;

struct ChannelProgressSink {
    task_id: String,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
}

impl ChannelProgressSink {
    fn emit(&self, completed: usize, total: usize, label: String) {
        let _ = self.event_tx.send(AppEvent::InstallProgress {
            task_id: self.task_id.clone(),
            completed,
            total,
            label,
        });
        self.ctx.request_repaint();
    }
}

impl InstallProgressSink for ChannelProgressSink {
    fn on_phase(&self, phase: InstallPhase) {
        let label = match phase {
            InstallPhase::DownloadingMinecraft { mc } => {
                format!("Downloading Minecraft {mc}")
            }
            InstallPhase::InstallingLoader {
                mc,
                loader,
                version,
            } => match version {
                Some(v) => format!("Installing {loader} {v} for MC {mc}"),
                None => format!("Installing latest {loader} for MC {mc}"),
            },
            InstallPhase::DownloadingMods { total } => {
                format!("Downloading {total} mods")
            }
            InstallPhase::WritingOverlay { total } => {
                format!("Writing {total} config files")
            }
            InstallPhase::Finalizing => "Finalizing instance".to_string(),
        };
        self.emit(0, 0, label);
    }

    fn on_mod_progress(&self, completed: usize, total: usize, current_name: &str) {
        let label = if completed >= total {
            "Mods downloaded".to_string()
        } else if current_name.is_empty() {
            format!("Mods ({completed}/{total})")
        } else {
            format!("Mod: {current_name}")
        };
        self.emit(completed, total, label);
    }

    fn on_overlay_progress(&self, completed: usize, total: usize, current_path: &str) {
        let label = if completed >= total {
            "Config written".to_string()
        } else if current_path.is_empty() {
            format!("Config ({completed}/{total})")
        } else {
            format!("Config: {current_path}")
        };
        self.emit(completed, total, label);
    }

    fn on_minecraft_progress(&self, completed: usize, total: usize, label: &str) {
        self.emit(completed, total, label.to_string());
    }
}

const FETCH_TIMEOUT: Duration = Duration::from_secs(8);
const RAW_GITHUB_PREFIX: &str = "https://raw.githubusercontent.com/";

fn jsdelivr_mirror(url: &str) -> Option<String> {
    let rest = url.strip_prefix(RAW_GITHUB_PREFIX)?;
    let (user, after_user) = rest.split_once('/')?;
    let (repo, after_repo) = after_user.split_once('/')?;
    let (branch, path) = after_repo.split_once('/')?;
    Some(format!(
        "https://cdn.jsdelivr.net/gh/{user}/{repo}@{branch}/{path}"
    ))
}

async fn fetch_with_github_fallback(url: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|e| format!("client init: {e}"))?;

    let primary_err = match try_fetch(&client, url).await {
        Ok(bytes) => return Ok(bytes),
        Err(e) => e,
    };

    let mirror = match jsdelivr_mirror(url) {
        Some(m) => m,
        None => return Err(primary_err),
    };

    tracing::info!(?url, mirror = ?mirror, "primary fetch failed, trying jsDelivr mirror");
    match try_fetch(&client, &mirror).await {
        Ok(bytes) => Ok(bytes),
        Err(mirror_err) => Err(format!(
            "both upstream and mirror failed.\n  upstream ({url}): {primary_err}\n  mirror ({mirror}): {mirror_err}"
        )),
    }
}

async fn try_fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| classify_reqwest_error(&e))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("body: {e}"))
}

fn classify_reqwest_error(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "connection timed out".to_string()
    } else if e.is_connect() {
        "connection refused or unreachable".to_string()
    } else {
        format!("{e}")
    }
}

pub fn handle_fetch_manifest(
    source_id: String,
    manifest_url: String,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = async {
            let raw = fetch_with_github_fallback(&manifest_url).await?;
            parse_manifest(&raw, &manifest_url).map_err(|e| format!("parse: {e}"))
        }
        .await;

        let event = match result {
            Ok(manifest) => AppEvent::ModpackManifestFetched {
                source_id,
                manifest,
            },
            Err(error) => AppEvent::ModpackManifestFailed { source_id, error },
        };
        let _ = event_tx.send(event);
        ctx.request_repaint();
    });
}

#[allow(clippy::too_many_arguments)]
pub fn handle_resolve_modpack(
    source_id: String,
    pack_id: String,
    pack_url: String,
    mc_version: String,
    _loader: String,
    instance_name: String,
    config: LauncherConfig,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = async {
            let pack_raw = fetch_with_github_fallback(&pack_url)
                .await
                .map_err(|e| format!("fetch pack.json: {e}"))?;
            let pack = parse_pack(&pack_raw).map_err(|e| format!("parse pack.json: {e}"))?;
            let http = Arc::new(ReqwestClient::new());
            let resolver_src = LiveResolverDataSource::new(http, None);
            let report = resolve(&pack, &pack_raw, &mc_version, &resolver_src)
                .await
                .map_err(|e| format!("resolve: {e}"))?;
            Ok::<_, String>((pack, pack_raw, report))
        }
        .await;

        let event = match result {
            Ok((pack, pack_raw, report)) => AppEvent::ModpackResolutionReady {
                source_id,
                pack_id,
                pack_url,
                instance_name,
                config: Box::new(config),
                report: Box::new(report),
                pack_raw,
                pack: Box::new(pack),
            },
            Err(error) => AppEvent::ModpackResolutionFailed {
                source_id,
                pack_id,
                error,
            },
        };
        let _ = event_tx.send(event);
        ctx.request_repaint();
    });
}

#[allow(clippy::too_many_arguments)]
pub fn handle_apply_install(
    source_id: String,
    pack_id: String,
    pack_url: String,
    instance_name: String,
    config: Box<LauncherConfig>,
    report: Box<miao_core::modpack_source::ResolutionReport>,
    pack_raw: Vec<u8>,
    pack: Box<miao_core::modpack_source::Pack>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    let config = *config;
    let report = *report;
    let pack = *pack;
    let task_id = format!("modpack-install:{source_id}:{pack_id}:{instance_name}");

    let _ = event_tx.send(AppEvent::InstallProgress {
        task_id: task_id.clone(),
        completed: 0,
        total: 0,
        label: format!("Installing {} mods…", report.mods.len()),
    });
    ctx.request_repaint();

    let sink = Arc::new(ChannelProgressSink {
        task_id: task_id.clone(),
        event_tx: event_tx.clone(),
        ctx: ctx.clone(),
    });

    tokio::spawn(async move {
        let progress_sink: Arc<dyn InstallProgressSink> = sink;
        let http = Arc::new(ReqwestClient::new());
        let executor = LiveInstallExecutor::new(http, Arc::new(config.clone()));

        let plan = InstallPlan {
            instance_name: instance_name.clone(),
            pack: &pack,
            pack_raw: &pack_raw,
            report: &report,
            source_id: source_id.clone(),
            source_url: pack_url.clone(),
            manifest_etag: None,
            progress: Some(progress_sink),
        };

        let instances_root = config.instances_dir().clone();
        let result = install(plan, &instances_root, &executor)
            .await
            .map_err(|e| format!("install: {e}"));

        let event = match result {
            Ok(_) => AppEvent::ModpackInstallFinished {
                task_id,
                source_id,
                pack_id,
                success: true,
                message: format!("Installed '{instance_name}'"),
            },
            Err(e) => AppEvent::ModpackInstallFinished {
                task_id,
                source_id,
                pack_id,
                success: false,
                message: e,
            },
        };
        let _ = event_tx.send(event);
        ctx.request_repaint();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_jsdelivr_mirror_main_branch() {
        assert_eq!(
            jsdelivr_mirror(
                "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/manifest.json"
            ),
            Some(
                "https://cdn.jsdelivr.net/gh/WangSimiao2000/miao-modpacks@main/manifest.json"
                    .to_string()
            )
        );
    }

    #[test]
    fn t_jsdelivr_mirror_nested_path() {
        assert_eq!(
            jsdelivr_mirror("https://raw.githubusercontent.com/u/r/v2/packs/abc/pack.json"),
            Some("https://cdn.jsdelivr.net/gh/u/r@v2/packs/abc/pack.json".to_string())
        );
    }

    #[test]
    fn t_jsdelivr_mirror_non_github_url_returns_none() {
        assert!(jsdelivr_mirror("https://example.com/manifest.json").is_none());
    }

    #[test]
    fn t_jsdelivr_mirror_malformed_path_returns_none() {
        assert!(jsdelivr_mirror("https://raw.githubusercontent.com/onlyuser").is_none());
    }
}
