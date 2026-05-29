use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("IO: {0}")]
    Io(#[from] std::io::Error),

    #[error("user sources file is malformed: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("source_id '{got}' is invalid (must match [a-z0-9-]{{2,32}})")]
    InvalidSourceId { got: String },

    #[error("manifest_url '{got}' must be https")]
    InvalidUrl { got: String },

    #[error("source_id '{id}' is already registered (built-in vs user collision is rejected)")]
    DuplicateId { id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackSource {
    pub source_id: String,
    pub manifest_url: String,
    pub is_built_in: bool,
    pub display_name_hint: Option<String>,
}

pub struct BuiltInSource {
    pub source_id: &'static str,
    pub manifest_url: &'static str,
    pub display_name_hint: &'static str,
}

pub const BUILT_IN_SOURCES: &[BuiltInSource] = &[BuiltInSource {
    source_id: "miao",
    manifest_url: "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/manifest.json",
    display_name_hint: "米奇喵整合包源",
}];

pub const USER_SOURCES_RELATIVE_PATH: &str = "modpack-sources/user.toml";

#[derive(Debug, Default, Deserialize)]
struct UserSourcesFile {
    #[serde(default)]
    sources: Vec<UserSource>,
}

#[derive(Debug, Deserialize)]
struct UserSource {
    source_id: String,
    manifest_url: String,
    #[serde(default)]
    display_name_hint: Option<String>,
}

pub fn load_sources(data_dir: &Path) -> Result<HashMap<String, ModpackSource>, RegistryError> {
    let mut sources: HashMap<String, ModpackSource> = HashMap::new();

    for built_in in BUILT_IN_SOURCES {
        sources.insert(
            built_in.source_id.to_string(),
            ModpackSource {
                source_id: built_in.source_id.to_string(),
                manifest_url: built_in.manifest_url.to_string(),
                is_built_in: true,
                display_name_hint: Some(built_in.display_name_hint.to_string()),
            },
        );
    }

    let user_path = data_dir.join(USER_SOURCES_RELATIVE_PATH);
    if user_path.exists() {
        let raw = std::fs::read_to_string(&user_path)?;
        let parsed: UserSourcesFile = toml::from_str(&raw)?;
        for u in parsed.sources {
            validate_source_id(&u.source_id)?;
            validate_https(&u.manifest_url)?;
            if sources.contains_key(&u.source_id) {
                return Err(RegistryError::DuplicateId {
                    id: u.source_id.clone(),
                });
            }
            sources.insert(
                u.source_id.clone(),
                ModpackSource {
                    source_id: u.source_id,
                    manifest_url: u.manifest_url,
                    is_built_in: false,
                    display_name_hint: u.display_name_hint,
                },
            );
        }
    }

    Ok(sources)
}

pub fn user_sources_path(data_dir: &Path) -> PathBuf {
    data_dir.join(USER_SOURCES_RELATIVE_PATH)
}

fn validate_source_id(s: &str) -> Result<(), RegistryError> {
    let re = regex::Regex::new(r"^[a-z0-9-]{2,32}$").expect("static regex");
    if re.is_match(s) {
        Ok(())
    } else {
        Err(RegistryError::InvalidSourceId { got: s.to_string() })
    }
}

fn validate_https(url: &str) -> Result<(), RegistryError> {
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err(RegistryError::InvalidUrl {
            got: url.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_registry_01_built_in_sources_always_loaded() {
        let dir = tempfile::tempdir().unwrap();
        let sources = load_sources(dir.path()).unwrap();
        assert!(sources.contains_key("miao"));
        assert!(sources["miao"].is_built_in);
        assert!(sources["miao"].manifest_url.starts_with("https://"));
    }

    #[test]
    fn t_registry_02_user_toml_missing_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let sources = load_sources(dir.path()).unwrap();
        assert_eq!(sources.len(), BUILT_IN_SOURCES.len());
    }

    #[test]
    fn t_registry_03_user_toml_loaded_alongside_builtins() {
        let dir = tempfile::tempdir().unwrap();
        let user_path = dir.path().join("modpack-sources/user.toml");
        std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
        std::fs::write(
            &user_path,
            r#"
[[sources]]
source_id = "third-party"
manifest_url = "https://example.com/manifest.json"
display_name_hint = "Third Party"
"#,
        )
        .unwrap();

        let sources = load_sources(dir.path()).unwrap();
        assert_eq!(sources.len(), BUILT_IN_SOURCES.len() + 1);
        assert!(sources.contains_key("third-party"));
        assert!(!sources["third-party"].is_built_in);
    }

    #[test]
    fn t_registry_04_user_source_with_invalid_id_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let user_path = dir.path().join("modpack-sources/user.toml");
        std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
        std::fs::write(
            &user_path,
            r#"
[[sources]]
source_id = "Bad_ID!"
manifest_url = "https://example.com/m.json"
"#,
        )
        .unwrap();

        let result = load_sources(dir.path());
        assert!(matches!(result, Err(RegistryError::InvalidSourceId { .. })));
    }

    #[test]
    fn t_registry_05_user_source_collides_with_builtin_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let user_path = dir.path().join("modpack-sources/user.toml");
        std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
        std::fs::write(
            &user_path,
            r#"
[[sources]]
source_id = "miao"
manifest_url = "https://attacker.example.com/m.json"
"#,
        )
        .unwrap();

        let result = load_sources(dir.path());
        assert!(matches!(result, Err(RegistryError::DuplicateId { .. })));
    }

    #[test]
    fn t_registry_06_user_source_with_http_url_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let user_path = dir.path().join("modpack-sources/user.toml");
        std::fs::create_dir_all(user_path.parent().unwrap()).unwrap();
        std::fs::write(
            &user_path,
            r#"
[[sources]]
source_id = "insecure"
manifest_url = "http://example.com/m.json"
"#,
        )
        .unwrap();

        let result = load_sources(dir.path());
        assert!(matches!(result, Err(RegistryError::InvalidUrl { .. })));
    }

    #[test]
    fn t_registry_07_user_sources_path_is_correct() {
        let dir = tempfile::tempdir().unwrap();
        let p = user_sources_path(dir.path());
        assert!(p.ends_with("modpack-sources/user.toml"));
    }
}
