use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
