use std::path::PathBuf;
use std::sync::Arc;

use egui::Context;
use tokio::sync::mpsc;

use crate::messages::AppEvent;
use crate::state::PendingModInstall;

pub fn handle_search(
    query: String,
    mc_version: String,
    loader: Option<String>,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = miao_core::modrinth::api::search_mods(
            &*http,
            &query,
            Some(&mc_version),
            loader.as_deref(),
            20,
        )
        .await;
        match result {
            Ok(r) => {
                let _ = tx.send(AppEvent::ModSearchResults(r.hits));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ModError(format!("Search failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_load_versions(
    slug: String,
    mc_version: String,
    loader: Option<String>,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = miao_core::modrinth::api::get_project_versions(
            &*http,
            &slug,
            Some(&mc_version),
            loader.as_deref(),
        )
        .await;
        match result {
            Ok(versions) => {
                let _ = tx.send(AppEvent::ModVersions(versions));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ModError(format!("Failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_resolve_deps(
    slug: String,
    mc_version: String,
    loader: String,
    instance_dir: PathBuf,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        match miao_core::modrinth::api::resolve_dependencies(&*http, &slug, &mc_version, &loader)
            .await
        {
            Ok(deps) => {
                let _ = tx.send(AppEvent::ModPendingInstall(PendingModInstall {
                    project_slug: slug,
                    deps,
                    instance_dir,
                    mc_version,
                    loader,
                }));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ModError(format!("Resolve failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_install_mod(
    pending: PendingModInstall,
    include_deps: bool,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let mods_dir = miao_core::instance::Instance::mods_dir(&pending.instance_dir);
        let result = if include_deps {
            miao_core::modrinth::api::install_mod_with_dependencies(
                &*http,
                &pending.project_slug,
                &pending.mc_version,
                &pending.loader,
                &mods_dir,
            )
            .await
        } else {
            miao_core::modrinth::api::install_mod_only(
                &*http,
                &pending.project_slug,
                &pending.mc_version,
                &pending.loader,
                &mods_dir,
            )
            .await
        };

        match result {
            Ok(results) => {
                let _ = tx.send(AppEvent::ModInstalled {
                    count: results.len(),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ModError(format!("Install failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_cf_search(
    query: String,
    mc_version: String,
    loader: Option<String>,
    api_key: String,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let client = miao_core::curseforge::api::CurseForgeClient::new(&api_key);
        let result = client
            .search_mods(&query, Some(&mc_version), loader.as_deref(), 20)
            .await;
        match result {
            Ok(r) => {
                let _ = tx.send(AppEvent::CfSearchResults(r.mods));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::CfError(format!(
                    "CurseForge search failed: {}",
                    e
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_cf_install(
    mod_id: u32,
    mc_version: String,
    loader: String,
    instance_dir: PathBuf,
    api_key: String,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let client = miao_core::curseforge::api::CurseForgeClient::new(&api_key);
        let mods_dir = miao_core::instance::Instance::mods_dir(&instance_dir);
        let result = client
            .install_mod(mod_id, &mc_version, &loader, &mods_dir)
            .await;
        match result {
            Ok(installed) => {
                let _ = tx.send(AppEvent::CfInstalled {
                    count: installed.len(),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::CfError(format!(
                    "CurseForge install failed: {}",
                    e
                )));
            }
        }
        ctx.request_repaint();
    });
}
