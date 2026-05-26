use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AuthError, Result};

const AUTHLIB_INJECTOR_DOWNLOAD_URL: &str =
    "https://authlib-injector.yushi.moe/artifact/latest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthlibInjectorAccount {
    pub username: String,
    pub uuid: Uuid,
    pub access_token: String,
    pub server_url: String,
    pub server_name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilAuthResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "selectedProfile")]
    selected_profile: YggdrasilProfile,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilProfile {
    id: String,
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct YggdrasilRefreshResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "selectedProfile")]
    selected_profile: YggdrasilProfile,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthlibInjectorArtifact {
    pub download_url: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ArtifactInfo {
    download_url: String,
    version: String,
}

pub async fn authenticate(
    http: &reqwest::Client,
    server_url: &str,
    email: &str,
    password: &str,
) -> Result<AuthlibInjectorAccount> {
    let auth_url = format!(
        "{}/authserver/authenticate",
        server_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "username": email,
        "password": password,
        "agent": {
            "name": "Minecraft",
            "version": 1
        }
    });

    let resp =
        http.post(&auth_url).json(&body).send().await.map_err(|e| {
            AuthError::MicrosoftAuth(format!("Yggdrasil auth request failed: {}", e))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = resp.text().await.unwrap_or_default();
        return Err(AuthError::MicrosoftAuth(format!(
            "Yggdrasil auth failed ({}): {}",
            status, error_body
        ))
        .into());
    }

    let auth_resp: YggdrasilAuthResponse = resp
        .json()
        .await
        .map_err(|e| AuthError::MicrosoftAuth(format!("Failed to parse auth response: {}", e)))?;

    let uuid = parse_yggdrasil_uuid(&auth_resp.selected_profile.id)?;
    let server_name = fetch_server_name(http, server_url)
        .await
        .unwrap_or_else(|_| server_url.to_string());

    Ok(AuthlibInjectorAccount {
        username: auth_resp.selected_profile.name,
        uuid,
        access_token: auth_resp.access_token,
        server_url: server_url.to_string(),
        server_name,
    })
}

pub async fn refresh(
    http: &reqwest::Client,
    account: &AuthlibInjectorAccount,
) -> Result<AuthlibInjectorAccount> {
    let refresh_url = format!(
        "{}/authserver/refresh",
        account.server_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "accessToken": account.access_token,
    });

    let resp = http
        .post(&refresh_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| AuthError::RefreshFailed(format!("Yggdrasil refresh failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(
            AuthError::RefreshFailed("Yggdrasil token refresh rejected".to_string()).into(),
        );
    }

    let refresh_resp: YggdrasilRefreshResponse = resp.json().await.map_err(|e| {
        AuthError::RefreshFailed(format!("Failed to parse refresh response: {}", e))
    })?;

    let uuid = parse_yggdrasil_uuid(&refresh_resp.selected_profile.id)?;

    Ok(AuthlibInjectorAccount {
        username: refresh_resp.selected_profile.name,
        uuid,
        access_token: refresh_resp.access_token,
        server_url: account.server_url.clone(),
        server_name: account.server_name.clone(),
    })
}

pub async fn validate(http: &reqwest::Client, account: &AuthlibInjectorAccount) -> Result<bool> {
    let validate_url = format!(
        "{}/authserver/validate",
        account.server_url.trim_end_matches('/')
    );

    let body = serde_json::json!({
        "accessToken": account.access_token,
    });

    let resp = http
        .post(&validate_url)
        .json(&body)
        .send()
        .await
        .map_err(|_| AuthError::RefreshFailed("Validation request failed".to_string()))?;

    Ok(resp.status().as_u16() == 204)
}

pub async fn fetch_latest_injector_artifact(
    http: &reqwest::Client,
) -> Result<AuthlibInjectorArtifact> {
    let info: ArtifactInfo = http
        .get(AUTHLIB_INJECTOR_DOWNLOAD_URL)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(AuthlibInjectorArtifact {
        download_url: info.download_url,
        version: info.version,
    })
}

pub async fn download_injector_jar(
    http: &reqwest::Client,
    artifact: &AuthlibInjectorArtifact,
    dest_dir: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let filename = format!("authlib-injector-{}.jar", artifact.version);
    let dest = dest_dir.join(&filename);

    if dest.exists() {
        return Ok(dest);
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bytes = http
        .get(&artifact.download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    std::fs::write(&dest, &bytes)?;
    Ok(dest)
}

pub fn build_jvm_arg(injector_jar_path: &std::path::Path, server_url: &str) -> String {
    format!("-javaagent:{}={}", injector_jar_path.display(), server_url)
}

async fn fetch_server_name(http: &reqwest::Client, server_url: &str) -> Result<String> {
    let url = server_url.trim_end_matches('/');

    #[derive(Deserialize)]
    struct ServerMeta {
        #[serde(rename = "serverName")]
        server_name: Option<String>,
        #[serde(rename = "meta")]
        meta: Option<MetaInfo>,
    }
    #[derive(Deserialize)]
    struct MetaInfo {
        #[serde(rename = "serverName")]
        server_name: Option<String>,
    }

    let resp: ServerMeta = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(resp
        .server_name
        .or_else(|| resp.meta.and_then(|m| m.server_name))
        .unwrap_or_else(|| server_url.to_string()))
}

fn parse_yggdrasil_uuid(id_str: &str) -> Result<Uuid> {
    let clean = id_str.replace('-', "");
    if clean.len() != 32 {
        return Err(crate::error::MiaoError::Other(format!(
            "Invalid UUID from Yggdrasil: {}",
            id_str
        )));
    }

    let formatted = format!(
        "{}-{}-{}-{}-{}",
        &clean[0..8],
        &clean[8..12],
        &clean[12..16],
        &clean[16..20],
        &clean[20..32]
    );
    Ok(Uuid::parse_str(&formatted)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_uuid_without_dashes() {
        let uuid = parse_yggdrasil_uuid("a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6").unwrap();
        assert_eq!(uuid.to_string(), "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6");
    }

    #[test]
    fn parse_uuid_with_dashes() {
        let uuid = parse_yggdrasil_uuid("a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6").unwrap();
        assert_eq!(uuid.to_string(), "a1b2c3d4-e5f6-a7b8-c9d0-e1f2a3b4c5d6");
    }

    #[test]
    fn parse_uuid_invalid() {
        assert!(parse_yggdrasil_uuid("short").is_err());
    }

    #[test]
    fn build_jvm_arg_format() {
        let path = std::path::Path::new("/tmp/authlib-injector-1.2.3.jar");
        let url = "https://littleskin.cn/api/yggdrasil";
        let arg = build_jvm_arg(path, url);
        assert_eq!(
            arg,
            "-javaagent:/tmp/authlib-injector-1.2.3.jar=https://littleskin.cn/api/yggdrasil"
        );
    }

    #[test]
    fn auth_response_deserializes() {
        let json = r#"{
            "accessToken": "token123",
            "clientToken": "client456",
            "selectedProfile": {
                "id": "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
                "name": "TestPlayer"
            }
        }"#;
        let resp: YggdrasilAuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "token123");
        assert_eq!(resp.selected_profile.name, "TestPlayer");
    }

    #[test]
    fn artifact_info_deserializes() {
        let json = r#"{
            "download_url": "https://example.com/authlib-injector-1.2.3.jar",
            "version": "1.2.3"
        }"#;
        let info: ArtifactInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.version, "1.2.3");
    }
}
