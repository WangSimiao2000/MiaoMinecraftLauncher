use serde::Deserialize;

use crate::config::LauncherConfig;
use crate::error::Result;
use crate::http::HttpClient;

const BMCLAPI_OPTIFINE_LIST: &str = "https://bmclapi2.bangbang93.com/optifine";

#[derive(Debug, Clone, Deserialize)]
pub struct OptiFineVersion {
    #[serde(rename = "mcversion")]
    pub mc_version: String,
    #[serde(rename = "type")]
    pub edition: String,
    pub patch: String,
    #[serde(default)]
    pub filename: String,
}

impl OptiFineVersion {
    pub fn display_name(&self) -> String {
        format!("OptiFine {} {}", self.edition, self.patch)
    }

    pub fn download_url(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            BMCLAPI_OPTIFINE_LIST, self.mc_version, self.edition, self.patch
        )
    }
}

pub async fn fetch_versions(
    http: &impl HttpClient,
    mc_version: &str,
) -> Result<Vec<OptiFineVersion>> {
    let url = format!("{}/{}", BMCLAPI_OPTIFINE_LIST, mc_version);
    let response = http.get_json::<Vec<OptiFineVersion>>(&url).await?;
    Ok(response)
}

pub async fn install_optifine(
    http: &impl HttpClient,
    version: &OptiFineVersion,
    config: &LauncherConfig,
) -> Result<std::path::PathBuf> {
    let download_url = version.download_url();
    let filename = if version.filename.is_empty() {
        format!("OptiFine_{}_{}.jar", version.edition, version.patch)
    } else {
        version.filename.clone()
    };

    let dest_dir = config.libraries_dir().join("optifine");
    std::fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(&filename);

    if dest_path.exists() {
        return Ok(dest_path);
    }

    let bytes = http.get_bytes(&download_url).await?;
    std::fs::write(&dest_path, &bytes)?;

    Ok(dest_path)
}

pub fn install_as_mod(
    optifine_jar: &std::path::Path,
    instance_dir: &std::path::Path,
) -> Result<()> {
    let mods_dir = instance_dir.join("mods");
    std::fs::create_dir_all(&mods_dir)?;

    let dest = mods_dir.join(
        optifine_jar
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
    );

    if !dest.exists() {
        std::fs::copy(optifine_jar, &dest)?;
    }

    Ok(())
}
