pub mod assets;
pub mod install;
pub mod manifest;
pub mod meta;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VersionType {
    Release,
    Snapshot,
    OldBeta,
    OldAlpha,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub id: String,
    pub version_type: VersionType,
    pub url: String,
    pub release_time: String,
}

impl VersionInfo {
    pub fn is_release(&self) -> bool {
        self.version_type == VersionType::Release
    }

    pub fn is_snapshot(&self) -> bool {
        self.version_type == VersionType::Snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_is_release() {
        let v = VersionInfo {
            id: "1.20.4".to_string(),
            version_type: VersionType::Release,
            url: String::new(),
            release_time: String::new(),
        };
        assert!(v.is_release());
        assert!(!v.is_snapshot());
    }

    #[test]
    fn version_info_is_snapshot() {
        let v = VersionInfo {
            id: "24w03a".to_string(),
            version_type: VersionType::Snapshot,
            url: String::new(),
            release_time: String::new(),
        };
        assert!(!v.is_release());
        assert!(v.is_snapshot());
    }
}
