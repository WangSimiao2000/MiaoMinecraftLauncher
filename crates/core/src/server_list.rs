use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
    pub port: u16,
}

impl Default for ServerEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            port: 25565,
        }
    }
}

impl ServerEntry {
    pub fn new(name: &str, address: &str) -> Self {
        Self {
            name: name.to_string(),
            address: address.to_string(),
            port: 25565,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn connection_string(&self) -> String {
        if self.port == 25565 {
            self.address.clone()
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerList {
    pub servers: Vec<ServerEntry>,
}

impl ServerList {
    pub fn add(&mut self, entry: ServerEntry) {
        self.servers.push(entry);
    }

    pub fn remove(&mut self, index: usize) -> Option<ServerEntry> {
        if index < self.servers.len() {
            Some(self.servers.remove(index))
        } else {
            None
        }
    }

    pub fn find_by_name(&self, name: &str) -> Option<&ServerEntry> {
        self.servers.iter().find(|s| s.name == name)
    }

    pub fn save_to(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn load_from(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_entry_default_port() {
        let entry = ServerEntry::default();
        assert_eq!(entry.port, 25565);
    }

    #[test]
    fn server_entry_new() {
        let entry = ServerEntry::new("Hypixel", "mc.hypixel.net");
        assert_eq!(entry.name, "Hypixel");
        assert_eq!(entry.address, "mc.hypixel.net");
        assert_eq!(entry.port, 25565);
    }

    #[test]
    fn server_entry_with_port() {
        let entry = ServerEntry::new("Custom", "localhost").with_port(25566);
        assert_eq!(entry.port, 25566);
    }

    #[test]
    fn connection_string_default_port() {
        let entry = ServerEntry::new("Test", "mc.example.com");
        assert_eq!(entry.connection_string(), "mc.example.com");
    }

    #[test]
    fn connection_string_custom_port() {
        let entry = ServerEntry::new("Test", "mc.example.com").with_port(25566);
        assert_eq!(entry.connection_string(), "mc.example.com:25566");
    }

    #[test]
    fn server_list_add_and_find() {
        let mut list = ServerList::default();
        list.add(ServerEntry::new("Hypixel", "mc.hypixel.net"));
        list.add(ServerEntry::new("Local", "localhost"));

        assert_eq!(list.servers.len(), 2);
        assert_eq!(
            list.find_by_name("Hypixel").unwrap().address,
            "mc.hypixel.net"
        );
        assert!(list.find_by_name("NotExist").is_none());
    }

    #[test]
    fn server_list_remove() {
        let mut list = ServerList::default();
        list.add(ServerEntry::new("A", "a.com"));
        list.add(ServerEntry::new("B", "b.com"));

        let removed = list.remove(0).unwrap();
        assert_eq!(removed.name, "A");
        assert_eq!(list.servers.len(), 1);
        assert_eq!(list.servers[0].name, "B");
    }

    #[test]
    fn server_list_remove_out_of_bounds() {
        let mut list = ServerList::default();
        assert!(list.remove(0).is_none());
        assert!(list.remove(99).is_none());
    }

    #[test]
    fn server_list_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("servers.toml");

        let mut list = ServerList::default();
        list.add(ServerEntry::new("Test", "test.com").with_port(12345));

        list.save_to(&path).unwrap();

        let loaded = ServerList::load_from(&path).unwrap();
        assert_eq!(loaded.servers.len(), 1);
        assert_eq!(loaded.servers[0].name, "Test");
        assert_eq!(loaded.servers[0].port, 12345);
    }

    #[test]
    fn server_list_load_nonexistent_returns_empty() {
        let path = PathBuf::from("/nonexistent/servers.toml");
        let list = ServerList::load_from(&path).unwrap();
        assert!(list.servers.is_empty());
    }

    #[test]
    fn server_entry_serialization_roundtrip() {
        let entry = ServerEntry::new("Roundtrip", "rtt.com").with_port(9999);
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ServerEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, deserialized);
    }
}
