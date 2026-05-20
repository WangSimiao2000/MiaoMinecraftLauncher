pub mod microsoft;
pub mod offline;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
