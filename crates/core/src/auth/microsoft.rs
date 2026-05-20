use anyhow::Result;

use super::MicrosoftAccount;

const MICROSOFT_AUTH_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

pub struct MicrosoftAuth {
    client_id: String,
    http: reqwest::Client,
}

impl MicrosoftAuth {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http: reqwest::Client::new(),
        }
    }

    pub fn get_auth_url(&self, redirect_uri: &str) -> String {
        format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&scope=XboxLive.signin%20offline_access",
            MICROSOFT_AUTH_URL, self.client_id, redirect_uri
        )
    }

    pub async fn authenticate_with_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<MicrosoftAccount> {
        todo!("Microsoft OAuth flow implementation")
    }

    pub async fn refresh(&self, _account: &MicrosoftAccount) -> Result<MicrosoftAccount> {
        todo!("Token refresh implementation")
    }
}
