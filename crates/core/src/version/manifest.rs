use crate::error::Result;
use serde::Deserialize;

use crate::config::DownloadMirror;
use crate::http::HttpClient;

use super::{VersionInfo, VersionType};

const MOJANG_VERSION_MANIFEST: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const BMCLAPI_VERSION_MANIFEST: &str =
    "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

#[derive(Debug, Deserialize)]
struct RawManifest {
    versions: Vec<RawVersion>,
}

#[derive(Debug, Deserialize)]
struct RawVersion {
    id: String,
    #[serde(rename = "type")]
    version_type: String,
    url: String,
    #[serde(rename = "releaseTime")]
    release_time: String,
}

pub async fn fetch_version_manifest(
    http: &impl HttpClient,
    mirror: &DownloadMirror,
) -> Result<Vec<VersionInfo>> {
    let url = match mirror {
        DownloadMirror::Bmclapi => BMCLAPI_VERSION_MANIFEST,
        _ => MOJANG_VERSION_MANIFEST,
    };

    let manifest: RawManifest = http.get_json(url).await?;

    let versions = manifest
        .versions
        .into_iter()
        .map(|v| VersionInfo {
            id: v.id,
            version_type: match v.version_type.as_str() {
                "release" => VersionType::Release,
                "snapshot" => VersionType::Snapshot,
                "old_beta" => VersionType::OldBeta,
                "old_alpha" => VersionType::OldAlpha,
                _ => VersionType::Snapshot,
            },
            url: v.url,
            release_time: v.release_time,
        })
        .collect();

    Ok(versions)
}
