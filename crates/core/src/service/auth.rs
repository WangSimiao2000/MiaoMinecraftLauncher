use crate::auth::AuthMethod;
use crate::auth::offline::create_offline_account;
use crate::error::Result;

use super::LauncherService;

impl LauncherService {
    pub async fn get_valid_account(&mut self) -> Result<AuthMethod> {
        use crate::auth::MS_CLIENT_ID;
        use crate::auth::microsoft::MicrosoftAuth;

        let idx = self.config.active_account_index.unwrap_or(0);
        let account = self
            .config
            .accounts
            .get(idx)
            .cloned()
            .unwrap_or_else(|| AuthMethod::Offline(create_offline_account("Player")));

        match account {
            AuthMethod::Microsoft(ref ms_acc) if ms_acc.is_expired() => {
                let auth = MicrosoftAuth::new(MS_CLIENT_ID.to_string());
                match auth.refresh(ms_acc).await {
                    Ok(refreshed) => {
                        let new_auth = AuthMethod::Microsoft(refreshed);
                        if let Some(stored) = self.config.accounts.get_mut(idx) {
                            *stored = new_auth.clone();
                        }
                        let _ = self.config.save();
                        Ok(new_auth)
                    }
                    Err(e) => Err(crate::error::MiaoError::Auth(
                        crate::error::AuthError::RefreshFailed(format!(
                            "Failed to refresh token for '{}': {}",
                            ms_acc.username, e
                        )),
                    )),
                }
            }
            _ => Ok(account),
        }
    }

    pub fn add_offline_account(&mut self, username: &str) -> Result<()> {
        let account = create_offline_account(username);
        self.config.accounts.push(AuthMethod::Offline(account));
        if self.config.active_account_index.is_none() {
            self.config.active_account_index = Some(0);
        }
        self.config.save()?;
        Ok(())
    }

    pub fn active_account(&self) -> Option<&AuthMethod> {
        self.config
            .active_account_index
            .and_then(|idx| self.config.accounts.get(idx))
    }
}
