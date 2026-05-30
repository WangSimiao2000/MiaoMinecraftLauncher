use crate::error::Result;
use serde::Deserialize;

use crate::config::DownloadMirror;
use crate::download::mirror::build_fallback_chain;
use crate::http::HttpClient;

use super::{VersionInfo, VersionType};

const MOJANG_VERSION_MANIFEST: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const BMCLAPI_VERSION_MANIFEST: &str =
    "https://bmclapi2.bangbang93.com/mc/game/version_manifest_v2.json";

fn manifest_url_for(mirror: &DownloadMirror) -> &'static str {
    match mirror {
        DownloadMirror::Bmclapi => BMCLAPI_VERSION_MANIFEST,
        _ => MOJANG_VERSION_MANIFEST,
    }
}

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
    let chain = build_fallback_chain(mirror);
    let mut last_err: Option<crate::error::MiaoError> = None;
    let mut manifest: Option<RawManifest> = None;
    for m in &chain {
        let url = manifest_url_for(m);
        match http.get_json::<RawManifest>(url).await {
            Ok(m) => {
                manifest = Some(m);
                break;
            }
            Err(e) => {
                tracing::warn!("version manifest fetch from {url} failed: {e}; trying fallback");
                last_err = Some(e);
            }
        }
    }
    let manifest = manifest.ok_or_else(|| {
        last_err.unwrap_or_else(|| {
            crate::error::MiaoError::Other("no mirrors in fallback chain".to_string())
        })
    })?;

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
