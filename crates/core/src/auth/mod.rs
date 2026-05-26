pub mod microsoft;
pub mod offline;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Microsoft OAuth client ID used for device code flow.
pub const MS_CLIENT_ID: &str = "d3bbcbda-1e98-4ccd-9fc7-b107f30a5af8";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Microsoft(MicrosoftAccount),
    Offline(OfflineAccount),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicrosoftAccount {
    pub username: String,
    pub uuid: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl MicrosoftAccount {
    /// Returns true if the Minecraft access token has expired or will expire within 5 minutes.
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() + chrono::Duration::minutes(5) >= self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineAccount {
    pub username: String,
    pub uuid: Uuid,
}

impl AuthMethod {
    pub fn username(&self) -> &str {
        match self {
            Self::Microsoft(acc) => &acc.username,
            Self::Offline(acc) => &acc.username,
        }
    }

    pub fn uuid(&self) -> &Uuid {
        match self {
            Self::Microsoft(acc) => &acc.uuid,
            Self::Offline(acc) => &acc.uuid,
        }
    }

    pub fn access_token(&self) -> &str {
        match self {
            Self::Microsoft(acc) => &acc.access_token,
            Self::Offline(_) => "0",
        }
    }

    pub fn is_microsoft(&self) -> bool {
        matches!(self, Self::Microsoft(_))
    }

    pub fn is_offline(&self) -> bool {
        matches!(self, Self::Offline(_))
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
