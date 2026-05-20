use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::MicrosoftAccount;

const MICROSOFT_TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const MICROSOFT_DEVICE_CODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const XBOX_AUTH_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MINECRAFT_AUTH_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MINECRAFT_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct XboxAuthRequest {
    #[serde(rename = "Properties")]
    properties: XboxAuthProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XboxAuthProperties {
    #[serde(rename = "AuthMethod")]
    auth_method: String,
    #[serde(rename = "SiteName")]
    site_name: String,
    #[serde(rename = "RpsTicket")]
    rps_ticket: String,
}

#[derive(Debug, Deserialize)]
struct XboxAuthResponse {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: XboxDisplayClaims,
}

#[derive(Debug, Deserialize)]
struct XboxDisplayClaims {
    xui: Vec<XboxXui>,
}

#[derive(Debug, Deserialize)]
struct XboxXui {
    uhs: String,
}

#[derive(Debug, Serialize)]
struct XstsRequest {
    #[serde(rename = "Properties")]
    properties: XstsProperties,
    #[serde(rename = "RelyingParty")]
    relying_party: String,
    #[serde(rename = "TokenType")]
    token_type: String,
}

#[derive(Debug, Serialize)]
struct XstsProperties {
    #[serde(rename = "SandboxId")]
    sandbox_id: String,
    #[serde(rename = "UserTokens")]
    user_tokens: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MinecraftAuthRequest {
    #[serde(rename = "identityToken")]
    identity_token: String,
}

#[derive(Debug, Deserialize)]
struct MinecraftAuthResponse {
    access_token: String,
    #[allow(dead_code)]
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct MinecraftProfile {
    id: String,
    name: String,
}

pub struct MicrosoftAuth {
    client_id: String,
    http: reqwest::Client,
}

#[derive(Debug)]
pub enum PollResult {
    Success(String, Option<String>),
    Pending,
    SlowDown,
    Expired,
    Error(String),
}

impl MicrosoftAuth {
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            http: reqwest::Client::new(),
        }
    }

    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("scope", "XboxLive.signin offline_access"),
        ];

        let resp = self
            .http
            .post(MICROSOFT_DEVICE_CODE_URL)
            .form(&params)
            .send()
            .await?
            .json::<DeviceCodeResponse>()
            .await
            .context("Failed to request device code")?;

        Ok(resp)
    }

    pub async fn poll_for_token(&self, device_code: &str) -> Result<PollResult> {
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", self.client_id.as_str()),
            ("device_code", device_code),
        ];

        let resp = self
            .http
            .post(MICROSOFT_TOKEN_URL)
            .form(&params)
            .send()
            .await?;
        let text = resp.text().await?;

        if let Ok(token) = serde_json::from_str::<TokenResponse>(&text) {
            return Ok(PollResult::Success(token.access_token, token.refresh_token));
        }

        if let Ok(err) = serde_json::from_str::<TokenErrorResponse>(&text) {
            return match err.error.as_str() {
                "authorization_pending" => Ok(PollResult::Pending),
                "slow_down" => Ok(PollResult::SlowDown),
                "expired_token" => Ok(PollResult::Expired),
                _ => Ok(PollResult::Error(err.error)),
            };
        }

        anyhow::bail!("Unexpected response from token endpoint: {}", text)
    }

    pub async fn authenticate_with_microsoft_token(
        &self,
        ms_access_token: &str,
        ms_refresh_token: Option<&str>,
    ) -> Result<MicrosoftAccount> {
        let xbox_token = self.xbox_authenticate(ms_access_token).await?;
        let (xsts_token, user_hash) = self.xsts_authenticate(&xbox_token).await?;
        let mc_token = self.minecraft_authenticate(&xsts_token, &user_hash).await?;
        let profile = self.get_minecraft_profile(&mc_token).await?;

        let uuid = parse_mojang_uuid(&profile.id)?;

        Ok(MicrosoftAccount {
            username: profile.name,
            uuid,
            access_token: mc_token,
            refresh_token: ms_refresh_token.unwrap_or_default().to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        })
    }

    pub async fn refresh(&self, account: &MicrosoftAccount) -> Result<MicrosoftAccount> {
        let params = [
            ("client_id", self.client_id.as_str()),
            ("refresh_token", account.refresh_token.as_str()),
            ("grant_type", "refresh_token"),
            ("scope", "XboxLive.signin offline_access"),
        ];

        let resp: TokenResponse = self
            .http
            .post(MICROSOFT_TOKEN_URL)
            .form(&params)
            .send()
            .await?
            .json()
            .await
            .context("Failed to refresh Microsoft token")?;

        self.authenticate_with_microsoft_token(&resp.access_token, resp.refresh_token.as_deref())
            .await
    }

    async fn xbox_authenticate(&self, ms_token: &str) -> Result<String> {
        let request = XboxAuthRequest {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: format!("d={}", ms_token),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };

        let resp: XboxAuthResponse = self
            .http
            .post(XBOX_AUTH_URL)
            .json(&request)
            .send()
            .await?
            .json()
            .await
            .context("Xbox Live authentication failed")?;

        Ok(resp.token)
    }

    async fn xsts_authenticate(&self, xbox_token: &str) -> Result<(String, String)> {
        let request = XstsRequest {
            properties: XstsProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec![xbox_token.to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let resp: XboxAuthResponse = self
            .http
            .post(XSTS_AUTH_URL)
            .json(&request)
            .send()
            .await?
            .json()
            .await
            .context("XSTS authentication failed")?;

        let user_hash = resp
            .display_claims
            .xui
            .first()
            .map(|x| x.uhs.clone())
            .unwrap_or_default();

        Ok((resp.token, user_hash))
    }

    async fn minecraft_authenticate(&self, xsts_token: &str, user_hash: &str) -> Result<String> {
        let request = MinecraftAuthRequest {
            identity_token: format!("XBL3.0 x={};{}", user_hash, xsts_token),
        };

        let resp: MinecraftAuthResponse = self
            .http
            .post(MINECRAFT_AUTH_URL)
            .json(&request)
            .send()
            .await?
            .json()
            .await
            .context("Minecraft authentication failed")?;

        Ok(resp.access_token)
    }

    async fn get_minecraft_profile(&self, mc_token: &str) -> Result<MinecraftProfile> {
        let resp: MinecraftProfile = self
            .http
            .get(MINECRAFT_PROFILE_URL)
            .bearer_auth(mc_token)
            .send()
            .await?
            .json()
            .await
            .context("Failed to get Minecraft profile")?;

        Ok(resp)
    }
}

pub fn parse_mojang_uuid(id: &str) -> Result<uuid::Uuid> {
    uuid::Uuid::parse_str(id).or_else(|_| {
        if id.len() != 32 {
            anyhow::bail!("Invalid UUID length: {}", id.len());
        }
        let with_dashes = format!(
            "{}-{}-{}-{}-{}",
            &id[..8],
            &id[8..12],
            &id[12..16],
            &id[16..20],
            &id[20..]
        );
        uuid::Uuid::parse_str(&with_dashes).context("Failed to parse UUID")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microsoft_auth_creates_with_client_id() {
        let auth = MicrosoftAuth::new("test-client-id".to_string());
        assert_eq!(auth.client_id, "test-client-id");
    }

    #[test]
    fn xbox_auth_request_serializes_correctly() {
        let request = XboxAuthRequest {
            properties: XboxAuthProperties {
                auth_method: "RPS".to_string(),
                site_name: "user.auth.xboxlive.com".to_string(),
                rps_ticket: "d=test_token".to_string(),
            },
            relying_party: "http://auth.xboxlive.com".to_string(),
            token_type: "JWT".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["Properties"]["AuthMethod"], "RPS");
        assert_eq!(json["Properties"]["RpsTicket"], "d=test_token");
        assert_eq!(json["RelyingParty"], "http://auth.xboxlive.com");
        assert_eq!(json["TokenType"], "JWT");
    }

    #[test]
    fn xsts_request_serializes_correctly() {
        let request = XstsRequest {
            properties: XstsProperties {
                sandbox_id: "RETAIL".to_string(),
                user_tokens: vec!["token123".to_string()],
            },
            relying_party: "rp://api.minecraftservices.com/".to_string(),
            token_type: "JWT".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["Properties"]["SandboxId"], "RETAIL");
        assert_eq!(json["Properties"]["UserTokens"][0], "token123");
    }

    #[test]
    fn minecraft_auth_request_serializes_correctly() {
        let request = MinecraftAuthRequest {
            identity_token: "XBL3.0 x=hash;xsts_token".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["identityToken"], "XBL3.0 x=hash;xsts_token");
    }

    #[test]
    fn device_code_response_deserializes() {
        let json = r#"{
            "device_code": "abc123",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://microsoft.com/devicelogin",
            "expires_in": 900,
            "interval": 5
        }"#;

        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "abc123");
        assert_eq!(resp.user_code, "ABCD-EFGH");
        assert_eq!(resp.verification_uri, "https://microsoft.com/devicelogin");
        assert_eq!(resp.expires_in, 900);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn xbox_auth_response_deserializes() {
        let json = r#"{
            "Token": "xbox_token_here",
            "DisplayClaims": {
                "xui": [{"uhs": "user_hash_123"}]
            }
        }"#;

        let resp: XboxAuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.token, "xbox_token_here");
        assert_eq!(resp.display_claims.xui[0].uhs, "user_hash_123");
    }

    #[test]
    fn minecraft_profile_deserializes() {
        let json = r#"{
            "id": "abcdef01234567890abcdef012345678",
            "name": "TestPlayer"
        }"#;

        let profile: MinecraftProfile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.id, "abcdef01234567890abcdef012345678");
        assert_eq!(profile.name, "TestPlayer");
    }

    #[test]
    fn parse_mojang_uuid_without_dashes() {
        let uuid = parse_mojang_uuid("abcdef01234567890abcdef012345678").unwrap();
        assert_eq!(
            uuid.to_string().replace('-', ""),
            "abcdef01234567890abcdef012345678"
        );
    }

    #[test]
    fn parse_mojang_uuid_with_dashes() {
        let uuid = parse_mojang_uuid("abcdef01-2345-6789-0abc-def012345678").unwrap();
        assert_eq!(uuid.to_string(), "abcdef01-2345-6789-0abc-def012345678");
    }

    #[test]
    fn parse_mojang_uuid_invalid() {
        assert!(parse_mojang_uuid("too_short").is_err());
    }
}
