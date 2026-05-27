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
    offset: u32,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let result = miao_core::modrinth::api::search_mods_offset(
            &*http,
            &query,
            Some(&mc_version),
            loader.as_deref(),
            20,
            offset,
        )
        .await;
        match result {
            Ok(r) => {
                let _ = tx.send(AppEvent::ModSearchResults {
                    hits: r.hits,
                    total_hits: r.total_hits,
                });
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
    index: u32,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let client = miao_core::curseforge::api::CurseForgeClient::new(&api_key);
        let result = client
            .search_mods_offset(&query, Some(&mc_version), loader.as_deref(), 20, index)
            .await;
        match result {
            Ok(r) => {
                let _ = tx.send(AppEvent::CfSearchResults {
                    mods: r.mods,
                    total_count: r.total_count,
                });
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

pub fn handle_cf_load_files(
    mod_id: u32,
    mc_version: String,
    loader: Option<String>,
    api_key: String,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let client = miao_core::curseforge::api::CurseForgeClient::new(&api_key);
        let result = client
            .get_mod_files(mod_id, Some(&mc_version), loader.as_deref())
            .await;
        match result {
            Ok(files) => {
                let _ = tx.send(AppEvent::CfFileVersions(files));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::CfError(format!(
                    "Failed to load CurseForge versions: {}",
                    e
                )));
            }
        }
        ctx.request_repaint();
    });
}

pub fn handle_cf_resolve_deps(
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
        let mod_info = client.get_mod(mod_id).await;
        let mod_name = mod_info
            .map(|m| m.name)
            .unwrap_or_else(|_| format!("Mod #{}", mod_id));

        match client
            .resolve_dependencies(mod_id, &mc_version, &loader)
            .await
        {
            Ok(deps) => {
                let _ = tx.send(AppEvent::CfPendingInstall {
                    mod_name,
                    deps,
                    instance_dir,
                    mc_version,
                    loader,
                    api_key,
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::CfError(format!(
                    "Failed to resolve dependencies: {}",
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

pub fn handle_check_mod_updates(
    mods_dir: std::path::PathBuf,
    mc_version: String,
    loader: String,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let mod_files = miao_core::modmanager::scan_mods_dir(&mods_dir);
        let mut hash_to_filename: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        for m in &mod_files {
            if !m.enabled {
                continue;
            }
            if let Ok(hash) = miao_core::modrinth::api::compute_sha1(&m.path) {
                hash_to_filename.insert(hash, m.file_name.clone());
            }
        }

        if hash_to_filename.is_empty() {
            let _ = tx.send(AppEvent::ModUpdatesResult { updates: vec![] });
            ctx.request_repaint();
            return;
        }

        let hashes: Vec<String> = hash_to_filename.keys().cloned().collect();
        let result =
            miao_core::modrinth::api::check_mod_updates(&http, &hashes, &mc_version, &loader).await;

        match result {
            Ok(update_map) => {
                let mut updates = Vec::new();
                for (hash, version) in update_map {
                    let filename = hash_to_filename.get(&hash).cloned().unwrap_or_default();
                    let file = version
                        .files
                        .iter()
                        .find(|f| f.primary)
                        .or(version.files.first());
                    if let Some(file) = file
                        && file.hashes.sha1.as_deref() != Some(&hash)
                    {
                        updates.push(miao_core::modrinth::api::ModUpdateInfo {
                            filename,
                            current_hash: hash,
                            new_version_name: version.name.clone(),
                            new_version_number: version.version_number.clone(),
                            new_file_url: file.url.clone(),
                            new_filename: file.filename.clone(),
                            project_id: version.project_id.clone(),
                        });
                    }
                }
                let _ = tx.send(AppEvent::ModUpdatesResult { updates });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::ModError(format!("Update check failed: {}", e)));
            }
        }
        ctx.request_repaint();
    });
}
