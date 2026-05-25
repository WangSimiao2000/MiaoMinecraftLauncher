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
