use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionMeta {
    pub id: String,
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    pub libraries: Vec<Library>,
    pub asset_index: AssetIndex,
    pub downloads: Downloads,
    pub java_version: Option<JavaVersion>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Arguments {
    pub game: Vec<serde_json::Value>,
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub total_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Downloads {
    pub client: DownloadEntry,
    pub server: Option<DownloadEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    pub major_version: u32,
}

impl VersionMeta {
    pub fn required_java_major(&self) -> u32 {
        self.java_version
            .as_ref()
            .map(|j| j.major_version)
            .unwrap_or(8)
    }

    pub fn is_library_allowed(library: &Library) -> bool {
        let Some(rules) = &library.rules else {
            return true;
        };

        let mut allowed = false;
        for rule in rules {
            let matches = match &rule.os {
                Some(os) => os.name.as_deref() == Some("linux"),
                None => true,
            };
            if matches {
                allowed = rule.action == "allow";
            }
        }
        allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_library(name: &str, rules: Option<Vec<Rule>>) -> Library {
        Library {
            name: name.to_string(),
            downloads: None,
            rules,
        }
    }

    #[test]
    fn library_no_rules_is_allowed() {
        let lib = make_library("com.mojang:authlib:1.0", None);
        assert!(VersionMeta::is_library_allowed(&lib));
    }

    #[test]
    fn library_allow_linux_is_allowed() {
        let lib = make_library(
            "org.lwjgl:lwjgl:3.3",
            Some(vec![Rule {
                action: "allow".to_string(),
                os: Some(OsRule {
                    name: Some("linux".to_string()),
                }),
            }]),
        );
        assert!(VersionMeta::is_library_allowed(&lib));
    }

    #[test]
    fn library_allow_windows_not_allowed() {
        let lib = make_library(
            "org.lwjgl:lwjgl:3.3",
            Some(vec![Rule {
                action: "allow".to_string(),
                os: Some(OsRule {
                    name: Some("windows".to_string()),
                }),
            }]),
        );
        assert!(!VersionMeta::is_library_allowed(&lib));
    }

    #[test]
    fn library_allow_all_then_disallow_osx() {
        let lib = make_library(
            "org.lwjgl:lwjgl:3.3",
            Some(vec![
                Rule {
                    action: "allow".to_string(),
                    os: None,
                },
                Rule {
                    action: "disallow".to_string(),
                    os: Some(OsRule {
                        name: Some("osx".to_string()),
                    }),
                },
            ]),
        );
        assert!(VersionMeta::is_library_allowed(&lib));
    }

    #[test]
    fn library_allow_all_then_disallow_linux() {
        let lib = make_library(
            "org.lwjgl:lwjgl-natives-macos:3.3",
            Some(vec![
                Rule {
                    action: "allow".to_string(),
                    os: None,
                },
                Rule {
                    action: "disallow".to_string(),
                    os: Some(OsRule {
                        name: Some("linux".to_string()),
                    }),
                },
            ]),
        );
        assert!(!VersionMeta::is_library_allowed(&lib));
    }

    #[test]
    fn required_java_major_with_version() {
        let meta = VersionMeta {
            id: "1.20.4".to_string(),
            main_class: String::new(),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![],
            asset_index: AssetIndex {
                id: "5".to_string(),
                sha1: String::new(),
                size: 0,
                url: String::new(),
                total_size: None,
            },
            downloads: Downloads {
                client: DownloadEntry {
                    sha1: String::new(),
                    size: 0,
                    url: String::new(),
                },
                server: None,
            },
            java_version: Some(JavaVersion { major_version: 17 }),
        };
        assert_eq!(meta.required_java_major(), 17);
    }

    #[test]
    fn required_java_major_defaults_to_8() {
        let meta = VersionMeta {
            id: "1.12.2".to_string(),
            main_class: String::new(),
            minecraft_arguments: None,
            arguments: None,
            libraries: vec![],
            asset_index: AssetIndex {
                id: "1.12".to_string(),
                sha1: String::new(),
                size: 0,
                url: String::new(),
                total_size: None,
            },
            downloads: Downloads {
                client: DownloadEntry {
                    sha1: String::new(),
                    size: 0,
                    url: String::new(),
                },
                server: None,
            },
            java_version: None,
        };
        assert_eq!(meta.required_java_major(), 8);
    }

    #[test]
    fn deserialize_version_meta_json() {
        let json = r#"{
            "id": "1.20.4",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [],
            "assetIndex": {
                "id": "12",
                "sha1": "abc",
                "size": 100,
                "url": "https://example.com/12.json",
                "totalSize": 500000
            },
            "downloads": {
                "client": {
                    "sha1": "def",
                    "size": 25000000,
                    "url": "https://example.com/client.jar"
                }
            },
            "javaVersion": {
                "majorVersion": 17
            }
        }"#;

        let meta: VersionMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.id, "1.20.4");
        assert_eq!(meta.main_class, "net.minecraft.client.main.Main");
        assert_eq!(meta.required_java_major(), 17);
        assert_eq!(meta.asset_index.id, "12");
        assert_eq!(meta.downloads.client.size, 25000000);
    }
}
