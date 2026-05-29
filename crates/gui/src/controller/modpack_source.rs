use std::sync::Arc;

use egui::Context;
use miao_core::config::LauncherConfig;
use miao_core::http::ReqwestClient;
use miao_core::modpack_source::{
    InstallPlan, LiveInstallExecutor, LiveResolverDataSource, install, parse_manifest, parse_pack,
    resolve,
};
use tokio::sync::mpsc;

use crate::messages::AppEvent;

pub fn handle_fetch_manifest(
    source_id: String,
    manifest_url: String,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = async {
            let resp = reqwest::Client::new()
                .get(&manifest_url)
                .send()
                .await
                .map_err(|e| format!("network: {e}"))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(format!("HTTP {status}"));
            }
            let raw = resp
                .bytes()
                .await
                .map_err(|e| format!("body: {e}"))?
                .to_vec();
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
    let client = reqwest::Client::new();
    let pack_raw = client
        .get(pack_url)
        .send()
        .await
        .map_err(|e| format!("fetch pack.json: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("read pack.json: {e}"))?
        .to_vec();
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
