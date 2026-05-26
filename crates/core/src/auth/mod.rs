pub mod authlib_injector;
pub mod microsoft;
pub mod offline;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use authlib_injector::AuthlibInjectorAccount;

pub const MS_CLIENT_ID: &str = "d3bbcbda-1e98-4ccd-9fc7-b107f30a5af8";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SkinModel {
    #[default]
    Classic,
    Slim,
}

impl SkinModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Slim => "slim",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Classic => "Classic (Steve)",
            Self::Slim => "Slim (Alex)",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
    AuthlibInjector(AuthlibInjectorAccount),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub username: String,
    pub uuid: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub skin_model: SkinModel,
}

impl MicrosoftAccount {
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() + chrono::Duration::minutes(5) >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub username: String,
    pub uuid: Uuid,
    #[serde(default)]
    pub skin_model: SkinModel,
}

impl AuthMethod {
    pub fn username(&self) -> &str {
        match self {
            Self::Microsoft(acc) => &acc.username,
            Self::Offline(acc) => &acc.username,
            Self::AuthlibInjector(acc) => &acc.username,
        }
    }

    pub fn uuid(&self) -> &Uuid {
        match self {
            Self::Microsoft(acc) => &acc.uuid,
            Self::Offline(acc) => &acc.uuid,
            Self::AuthlibInjector(acc) => &acc.uuid,
        }
    }

    pub fn access_token(&self) -> &str {
        match self {
            Self::Microsoft(acc) => &acc.access_token,
            Self::Offline(_) => "0",
            Self::AuthlibInjector(acc) => &acc.access_token,
        }
    }

    pub fn is_microsoft(&self) -> bool {
        matches!(self, Self::Microsoft(_))
    }

    pub fn is_offline(&self) -> bool {
        matches!(self, Self::Offline(_))
    }

    pub fn is_authlib_injector(&self) -> bool {
        matches!(self, Self::AuthlibInjector(_))
    }

    pub fn authlib_injector_server_url(&self) -> Option<&str> {
        match self {
            Self::AuthlibInjector(acc) => Some(&acc.server_url),
            _ => None,
        }
    }

    pub fn skin_model(&self) -> SkinModel {
        match self {
            Self::Microsoft(acc) => acc.skin_model,
            Self::Offline(acc) => acc.skin_model,
            Self::AuthlibInjector(_) => SkinModel::Classic,
        }
    }

    pub fn avatar_url(&self) -> String {
        let uuid = self.uuid().as_simple().to_string();
        format!("https://mc-heads.net/avatar/{}/64", uuid)
    }

    pub fn cape_url(&self) -> Option<String> {
        if self.is_offline() {
            return None;
        }
        let uuid = self.uuid().as_simple().to_string();
        Some(format!("https://crafatar.com/capes/{}", uuid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use offline::create_offline_account;

    #[test]
    fn auth_method_offline_accessors() {
        let account = create_offline_account("TestUser");
        let auth = AuthMethod::Offline(account);

        assert_eq!(auth.username(), "TestUser");
        assert_eq!(auth.access_token(), "0");
        assert!(auth.is_offline());
        assert!(!auth.is_microsoft());
    }

    #[test]
    fn auth_method_microsoft_accessors() {
        let account = MicrosoftAccount {
            username: "MSPlayer".to_string(),
            uuid: Uuid::new_v4(),
            access_token: "token123".to_string(),
            refresh_token: "refresh456".to_string(),
            expires_at: chrono::Utc::now(),
            skin_model: SkinModel::Classic,
        };
        let auth = AuthMethod::Microsoft(account);

        assert_eq!(auth.username(), "MSPlayer");
        assert_eq!(auth.access_token(), "token123");
        assert!(auth.is_microsoft());
        assert!(!auth.is_offline());
    }

    #[test]
    fn auth_method_serialization_roundtrip() {
        let account = create_offline_account("SerializeTest");
        let auth = AuthMethod::Offline(account);

        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: AuthMethod = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.username(), "SerializeTest");
        assert!(deserialized.is_offline());
    }
}
