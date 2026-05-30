use std::sync::Arc;
use std::time::Duration;

use egui::Context;
use miao_core::config::LauncherConfig;
use miao_core::http::ReqwestClient;
use miao_core::modpack_source::{
    InstallPlan, LiveInstallExecutor, LiveResolverDataSource, install, parse_manifest, parse_pack,
    resolve,
};
use tokio::sync::mpsc;

use crate::messages::AppEvent;

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
pub fn handle_install_modpack(
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
        let result = run_install(
            &source_id,
            &pack_id,
            &pack_url,
            &mc_version,
            &instance_name,
            &config,
        )
        .await;

        let event = match result {
            Ok(_) => AppEvent::ModpackInstallFinished {
                source_id,
                pack_id,
                success: true,
                message: format!("installed '{instance_name}'"),
            },
            Err(e) => AppEvent::ModpackInstallFinished {
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

async fn run_install(
    source_id: &str,
    _pack_id: &str,
    pack_url: &str,
    mc_version: &str,
    instance_name: &str,
    config: &LauncherConfig,
) -> Result<(), String> {
    let pack_raw = fetch_with_github_fallback(pack_url)
        .await
        .map_err(|e| format!("fetch pack.json: {e}"))?;
    let pack = parse_pack(&pack_raw).map_err(|e| format!("parse pack.json: {e}"))?;

    let http = Arc::new(ReqwestClient::new());
    let cf = None;
    let resolver_src = LiveResolverDataSource::new(http.clone(), cf);
    let report = resolve(&pack, &pack_raw, mc_version, &resolver_src)
        .await
        .map_err(|e| format!("resolve: {e}"))?;

    let executor = LiveInstallExecutor::new(http, Arc::new(config.clone()));
    let plan = InstallPlan {
        instance_name: instance_name.to_string(),
        pack: &pack,
        pack_raw: &pack_raw,
        report: &report,
        source_id: source_id.to_string(),
        source_url: pack_url.to_string(),
        manifest_etag: None,
    };

    let instances_root = config.instances_dir().clone();
    install(plan, &instances_root, &executor)
        .await
        .map_err(|e| format!("install: {e}"))?;

    Ok(())
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
