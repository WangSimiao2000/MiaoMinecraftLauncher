use serde::Deserialize;

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Arguments {
    pub game: Vec<serde_json::Value>,
    pub jvm: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibraryDownloads>,
    pub rules: Option<Vec<Rule>>,
}

#[derive(Debug, Deserialize)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
}

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
}

#[derive(Debug, Deserialize)]
pub struct OsRule {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
    pub total_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Downloads {
    pub client: DownloadEntry,
    pub server: Option<DownloadEntry>,
}

#[derive(Debug, Deserialize)]
pub struct DownloadEntry {
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Deserialize)]
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
