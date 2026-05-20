use uuid::Uuid;

use super::OfflineAccount;

pub fn create_offline_account(username: &str) -> OfflineAccount {
    let uuid = Uuid::new_v3(
        &Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", username).as_bytes(),
    );
    OfflineAccount {
        username: username.to_string(),
        uuid,
    }
}
