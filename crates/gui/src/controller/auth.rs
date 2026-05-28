use std::sync::Arc;

use egui::Context;
use miao_core::auth::AuthMethod;
use miao_core::auth::microsoft::{MicrosoftAuth, PollResult};
use miao_core::config::LauncherConfig;
use tokio::sync::mpsc;

use crate::messages::AppEvent;

pub fn handle_ms_login(
    client_id: String,
    mut config: LauncherConfig,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let auth = MicrosoftAuth::new(client_id);
        let device_code = match auth.request_device_code().await {
            Ok(dc) => dc,
            Err(e) => {
                let _ = tx.send(AppEvent::LoginFailed(format!("Login error: {}", e)));
                ctx.request_repaint();
                return;
            }
        };

        let _ = open::that(&device_code.verification_uri);
        let code = device_code.device_code.clone();
        let interval = device_code.interval;
        let _ = tx.send(AppEvent::DeviceCode(device_code));
        ctx.request_repaint();

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            match auth.poll_for_token(&code).await {
                Ok(PollResult::Success(access_token, refresh_token)) => {
                    match auth
                        .authenticate_with_microsoft_token(&access_token, refresh_token.as_deref())
                        .await
                    {
                        Ok(account) => {
                            config.accounts.push(AuthMethod::Microsoft(account.clone()));
                            if config.active_account_index.is_none() {
                                config.active_account_index = Some(0);
                            }
                            let _ = config.save();

                            let http = reqwest::Client::new();
                            let uuid = account.uuid.as_simple().to_string();
                            let cache = miao_core::skin::SkinCache::new(&config.data_dir);
                            let _ = miao_core::skin::fetch_and_cache_textures(&http, &uuid, &cache)
                                .await;

                            let _ = tx.send(AppEvent::LoginComplete {
                                account: AuthMethod::Microsoft(account),
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::LoginFailed(format!("Auth error: {}", e)));
                        }
                    }
                    ctx.request_repaint();
                    return;
                }
                Ok(PollResult::Pending) | Ok(PollResult::SlowDown) => continue,
                Ok(PollResult::Expired) => {
                    let _ = tx.send(AppEvent::LoginFailed("Code expired.".to_string()));
                    ctx.request_repaint();
                    return;
                }
                Ok(PollResult::Error(e)) => {
                    let _ = tx.send(AppEvent::LoginFailed(format!("Error: {}", e)));
                    ctx.request_repaint();
                    return;
                }
                Err(_) => continue,
            }
        }
    });
}

pub fn handle_refresh_skin(
    uuid: String,
    data_dir: std::path::PathBuf,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        let http = reqwest::Client::new();
        let cache = miao_core::skin::SkinCache::new(&data_dir);
        // Best-effort: ignore failures (Mojang outage, no internet, etc.). The UI
        // already rendered a placeholder; the next refresh will try again.
        let _ = miao_core::skin::fetch_and_cache_textures(&http, &uuid, &cache).await;
        let _ = tx.send(AppEvent::SkinRefreshed { uuid });
        ctx.request_repaint();
    });
}

pub fn handle_authlib_login(
    server_url: String,
    email: String,
    password: String,
    http: Arc<reqwest::Client>,
    tx: mpsc::UnboundedSender<AppEvent>,
    ctx: Context,
) {
    tokio::spawn(async move {
        match miao_core::auth::authlib_injector::authenticate(&http, &server_url, &email, &password)
            .await
        {
            Ok(account) => {
                let _ = tx.send(AppEvent::LoginComplete {
                    account: AuthMethod::AuthlibInjector(account),
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::LoginFailed(format!(
                    "Authlib login failed: {}",
                    e
                )));
            }
        }
        ctx.request_repaint();
    });
}
