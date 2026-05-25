pub mod auth;
pub mod instance;
pub mod java;
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
    let mut active_task: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            AppCommand::CreateInstance {
                ver,
                name,
                loader,
                config,
            } => {
                active_task = Some(instance::handle_create_instance(
                    ver,
                    name,
                    loader,
                    config,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                ));
            }
            AppCommand::LaunchInstance {
                idx,
                instance,
                config,
            } => {
                instance::handle_launch_instance(
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
            AppCommand::ImportMrpack { config } => {
                instance::handle_import_mrpack(config, event_tx.clone(), ctx.clone());
            }
            AppCommand::StartMsLogin { client_id, config } => {
                auth::handle_ms_login(client_id, config, event_tx.clone(), ctx.clone());
            }
            AppCommand::DownloadJava {
                required_major,
                java_dir,
                launch_idx,
            } => {
                active_task = Some(java::handle_download_java(
                    required_major,
                    java_dir,
                    launch_idx,
                    http.clone(),
                    event_tx.clone(),
                    ctx.clone(),
                ));
            }
            AppCommand::FetchVersionManifest { mirror } => {
                versions::handle_fetch_manifest(mirror, http.clone(), event_tx.clone(), ctx.clone());
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
            } => {
                mods::handle_search(
                    query,
                    mc_version,
                    loader,
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
            AppCommand::CheckForUpdates => {
                versions::handle_check_updates(event_tx.clone(), ctx.clone());
            }
            AppCommand::CancelCurrentTask => {
                if let Some(handle) = active_task.take() {
                    handle.abort();
                    let _ = event_tx.send(AppEvent::TaskCancelled);
                    ctx.request_repaint();
                }
            }
        }
    }
}
