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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_account_deterministic_uuid() {
        let acc1 = create_offline_account("Steve");
        let acc2 = create_offline_account("Steve");
        assert_eq!(acc1.uuid, acc2.uuid);
        assert_eq!(acc1.username, "Steve");
    }

    #[test]
    fn offline_account_different_names_different_uuids() {
        let acc1 = create_offline_account("Steve");
        let acc2 = create_offline_account("Alex");
        assert_ne!(acc1.uuid, acc2.uuid);
    }

    #[test]
    fn offline_account_uuid_is_v3() {
        let acc = create_offline_account("TestPlayer");
        assert_eq!(acc.uuid.get_version_num(), 3);
    }
}
