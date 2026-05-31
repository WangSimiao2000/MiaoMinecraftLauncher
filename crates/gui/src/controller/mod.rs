pub mod auth;
pub mod instance;
pub mod java;
pub mod modpack_source;
pub mod mods;
pub mod versions;

use std::sync::Arc;

use egui::Context;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::messages::{AppCommand, AppEvent};

pub struct AppController {
    cmd_tx: mpsc::UnboundedSender<AppCommand>,
    event_rx: mpsc::UnboundedReceiver<AppEvent>,
    _loop_handle: tokio::task::JoinHandle<()>,
}

impl AppController {
    pub fn new(rt: &Arc<Runtime>, ctx: Context) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let http = Arc::new(reqwest::Client::new());
        let handle = rt.spawn(controller_loop(cmd_rx, event_tx, ctx, http));

        Self {
            cmd_tx,
            event_rx,
            _loop_handle: handle,
        }
    }

    pub fn send(&self, cmd: AppCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    pub fn try_recv(&mut self) -> Option<AppEvent> {
        self.event_rx.try_recv().ok()
    }
}

async fn controller_loop(
    mut cmd_rx: mpsc::UnboundedReceiver<AppCommand>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
    http: Arc<reqwest::Client>,
) {
    let mut active_tasks: std::collections::HashMap<String, tokio::task::JoinHandle<()>> =
        std::collections::HashMap::new();

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            AppCommand::CreateInstance {
                task_id,
                ver,
                name,
                loader,
                config,
            } => {
                let handle = instance::handle_create_instance(
                    task_id.clone(),
                    ver,
                    name,
                    loader,
                    config,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
                active_tasks.insert(task_id, handle);
            }
            AppCommand::LaunchInstance {
                task_id,
                idx,
                instance,
                config,
            } => {
                instance::handle_launch_instance(
                    task_id,
                    idx,
                    instance,
                    config,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::ExportInstance { instance, config } => {
                instance::handle_export_instance(instance, config, event_tx.clone(), ctx.clone());
            }
            AppCommand::ImportMrpack { task_id, config } => {
                instance::handle_import_mrpack(task_id, config, event_tx.clone(), ctx.clone());
            }
            AppCommand::FetchModpackManifest {
                source_id,
                manifest_url,
            } => {
                modpack_source::handle_fetch_manifest(
                    source_id,
                    manifest_url,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::ResolveModpack {
                source_id,
                pack_id,
                pack_url,
                mc_version,
                loader,
                instance_name,
                config,
            } => {
                modpack_source::handle_resolve_modpack(
                    source_id,
                    pack_id,
                    pack_url,
                    mc_version,
                    loader,
                    instance_name,
                    config,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::ApplyModpackInstall {
                source_id,
                pack_id,
                pack_url,
                instance_name,
                config,
                report,
                pack_raw,
                pack,
            } => {
                modpack_source::handle_apply_install(
                    source_id,
                    pack_id,
                    pack_url,
                    instance_name,
                    config,
                    report,
                    pack_raw,
                    pack,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::StartMsLogin { client_id, config } => {
                auth::handle_ms_login(client_id, config, event_tx.clone(), ctx.clone());
            }
            AppCommand::StartAuthlibLogin {
                server_url,
                email,
                password,
            } => {
                auth::handle_authlib_login(
                    server_url,
                    email,
                    password,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::RefreshSkin { uuid, data_dir } => {
                auth::handle_refresh_skin(uuid, data_dir, event_tx.clone(), ctx.clone());
            }
            AppCommand::DownloadJava {
                task_id,
                required_major,
                launch_idx,
                config,
            } => {
                let handle = java::handle_download_java(
                    task_id.clone(),
                    required_major,
                    launch_idx,
                    config,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
                active_tasks.insert(task_id, handle);
            }
            AppCommand::RefreshJava { data_dir, force } => {
                java::handle_refresh_java(data_dir, force, event_tx.clone(), ctx.clone());
            }
            AppCommand::FetchVersionManifest { mirror } => {
                versions::handle_fetch_manifest(
                    mirror,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::FetchLoaderVersions { mc_version } => {
                versions::handle_fetch_loader_versions(
                    mc_version,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::SearchMods {
                query,
                mc_version,
                loader,
                offset,
            } => {
                mods::handle_search(
                    query,
                    mc_version,
                    loader,
                    offset,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::LoadModVersions {
                slug,
                mc_version,
                loader,
            } => {
                mods::handle_load_versions(
                    slug,
                    mc_version,
                    loader,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::ResolveDeps {
                slug,
                mc_version,
                loader,
                instance_dir,
            } => {
                mods::handle_resolve_deps(
                    slug,
                    mc_version,
                    loader,
                    instance_dir,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::InstallMod {
                pending,
                include_deps,
            } => {
                mods::handle_install_mod(
                    pending,
                    include_deps,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CfSearchMods {
                query,
                mc_version,
                loader,
                api_key,
                index,
            } => {
                mods::handle_cf_search(
                    query,
                    mc_version,
                    loader,
                    api_key,
                    index,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CfLoadFiles {
                mod_id,
                mc_version,
                loader,
                api_key,
            } => {
                mods::handle_cf_load_files(
                    mod_id,
                    mc_version,
                    loader,
                    api_key,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CfResolveDeps {
                mod_id,
                mc_version,
                loader,
                instance_dir,
                api_key,
            } => {
                mods::handle_cf_resolve_deps(
                    mod_id,
                    mc_version,
                    loader,
                    instance_dir,
                    api_key,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CfInstallMod {
                mod_id,
                mc_version,
                loader,
                instance_dir,
                api_key,
            } => {
                mods::handle_cf_install(
                    mod_id,
                    mc_version,
                    loader,
                    instance_dir,
                    api_key,
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CheckModUpdates {
                mods_dir,
                mc_version,
                loader,
            } => {
                mods::handle_check_mod_updates(
                    mods_dir,
                    mc_version,
                    loader,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                );
            }
            AppCommand::CheckForUpdates => {
                versions::handle_check_updates(event_tx.clone(), ctx.clone());
            }
            AppCommand::CancelTask { task_id } => {
                if let Some(handle) = active_tasks.remove(&task_id) {
                    handle.abort();
                    let _ = event_tx.send(AppEvent::TaskCancelled { task_id });
                    ctx.request_repaint();
                }
            }
        }
    }
}
