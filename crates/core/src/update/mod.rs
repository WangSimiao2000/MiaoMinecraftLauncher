use std::path::PathBuf;

use crate::error::Result;
use serde::Deserialize;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/WangSimiao2000/MiaoMinecraftLauncher/releases/latest";
const GITHUB_REPO_OWNER: &str = "WangSimiao2000";
const GITHUB_REPO_NAME: &str = "MiaoMinecraftLauncher";

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub html_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_notes: Option<String>,
    pub download_url: String,
    pub asset_name: String,
    pub asset_size: u64,
    pub release_page_url: String,
}

pub async fn check_for_update(http: &reqwest::Client) -> Result<Option<UpdateInfo>> {
    let resp: ReleaseInfo = http
        .get(GITHUB_API_URL)
        .header(
            "User-Agent",
            concat!("MiaoMinecraftLauncher/", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let current = env!("CARGO_PKG_VERSION");
    let latest = resp.tag_name.trim_start_matches('v');

    if latest == current {
        return Ok(None);
    }

    let platform_asset = find_platform_asset(&resp.assets);
    let Some(asset) = platform_asset else {
        return Ok(None);
    };

    Ok(Some(UpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        release_notes: resp.body,
        download_url: asset.browser_download_url.clone(),
        asset_name: asset.name.clone(),
        asset_size: asset.size,
        release_page_url: resp.html_url,
    }))
}

pub async fn download_update(
    http: &reqwest::Client,
    update_info: &UpdateInfo,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    let dest = dest_dir.join(&update_info.asset_name);

    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let bytes = http
        .get(&update_info.download_url)
        .header(
            "User-Agent",
            concat!("MiaoMinecraftLauncher/", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    tokio::fs::write(&dest, &bytes).await?;
    Ok(dest)
}

pub fn apply_update(downloaded_path: &std::path::Path) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let tmp = current_exe.with_extension("tmp");
    self_update::Move::from_source(downloaded_path)
        .replace_using_temp(&tmp)
        .to_dest(&current_exe)
        .map_err(|e| crate::error::MiaoError::Other(format!("Failed to apply update: {}", e)))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        let _ = std::fs::set_permissions(&current_exe, perms);
    }

    Ok(())
}

pub fn perform_self_update() -> std::result::Result<self_update::Status, crate::error::MiaoError> {
    let bin_name = if cfg!(target_os = "windows") {
        "mmcl-windows-x86_64.exe"
    } else {
        "mmcl-linux-x86_64.AppImage"
    };

    let status = self_update::backends::github::Update::configure()
        .repo_owner(GITHUB_REPO_OWNER)
        .repo_name(GITHUB_REPO_NAME)
        .bin_name(bin_name)
        .target("")
        .current_version(self_update::cargo_crate_version!())
        .no_confirm(true)
        .build()
        .map_err(|e| crate::error::MiaoError::Other(format!("Update config error: {}", e)))?
        .update()
        .map_err(|e| crate::error::MiaoError::Other(format!("Update failed: {}", e)))?;
    Ok(status)
}

fn find_platform_asset(assets: &[ReleaseAsset]) -> Option<&ReleaseAsset> {
    let target = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        return None;
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };

    let gui_ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ".appimage"
    };

    assets
        .iter()
        .find(|a| {
            let name_lower = a.name.to_lowercase();
            name_lower.contains(target)
                && name_lower.contains(arch)
                && name_lower.ends_with(gui_ext)
                && !name_lower.contains("cli")
        })
        .or_else(|| {
            assets.iter().find(|a| {
                let name_lower = a.name.to_lowercase();
                name_lower.contains(target)
                    && name_lower.contains(arch)
                    && !name_lower.ends_with(".sha256")
                    && !name_lower.ends_with(".sig")
                    && !name_lower.ends_with(".asc")
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_info_deserializes() {
        let json = r###"{
            "tag_name": "v0.2.0",
            "name": "MMCL v0.2.0",
            "body": "Changes: Feature A, Feature B",
            "html_url": "https://github.com/WangSimiao2000/MiaoMinecraftLauncher/releases/tag/v0.2.0",
            "assets": [{
                "name": "mmcl-linux-x86_64.tar.gz",
                "browser_download_url": "https://github.com/releases/download/v0.2.0/mmcl-linux-x86_64.tar.gz",
                "size": 10485760,
                "content_type": "application/gzip"
            }]
        }"###;
        let info: ReleaseInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.tag_name, "v0.2.0");
        assert_eq!(info.assets.len(), 1);
        assert_eq!(info.assets[0].name, "mmcl-linux-x86_64.tar.gz");
    }

    #[test]
    fn find_platform_asset_linux() {
        let assets = vec![
            ReleaseAsset {
                name: "mmcl-windows-x86_64.exe".to_string(),
                browser_download_url: "https://example.com/win".to_string(),
                size: 1000,
                content_type: "application/octet-stream".to_string(),
            },
            ReleaseAsset {
                name: "mmcl-cli-linux-x86_64".to_string(),
                browser_download_url: "https://example.com/cli".to_string(),
                size: 2000,
                content_type: "application/octet-stream".to_string(),
            },
            ReleaseAsset {
                name: "mmcl-linux-x86_64.AppImage".to_string(),
                browser_download_url: "https://example.com/appimage".to_string(),
                size: 5000,
                content_type: "application/octet-stream".to_string(),
            },
        ];

        let found = find_platform_asset(&assets);
        assert!(found.is_some());
        #[cfg(target_os = "linux")]
        assert!(found.unwrap().name.contains(".AppImage"));
        #[cfg(target_os = "windows")]
        assert!(found.unwrap().name.contains(".exe"));
    }

    #[test]
    fn find_platform_asset_not_found() {
        let assets = vec![ReleaseAsset {
            name: "mmcl-freebsd-x86_64".to_string(),
            browser_download_url: "https://example.com/freebsd".to_string(),
            size: 2000,
            content_type: "application/octet-stream".to_string(),
        }];

        let found = find_platform_asset(&assets);
        #[cfg(not(target_os = "freebsd"))]
        assert!(found.is_none());
    }
}
