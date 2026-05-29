//! Mojang JRE source.
//!
//! Implements [`crate::java::install::JavaSource::Mojang`] by translating the official
//! Minecraft Java runtime manifest into a generic [`JavaInstallPlan`]. The actual
//! download/install lives in [`crate::java::install`] and is shared across sources.
//!
//! Compared to fetching from third-party redistributors directly, this strategy:
//!
//! 1. Goes through Mojang's CDN (much more reliable from mainland China than GitHub).
//! 2. Is mirrored host-for-host by BMCLAPI (`piston-meta.mojang.com` and
//!    `piston-data.mojang.com` → `bmclapi2.bangbang93.com`), so the existing
//!    [`crate::download`] mirror infrastructure handles fallback automatically.
//! 3. Streams files individually with SHA-1 verification — a mid-file disconnect only
//!    requires re-fetching that single file, not the whole archive.

use std::collections::HashMap;

use serde::Deserialize;

use crate::config::{JavaSource, LauncherConfig};
use crate::download::DownloadTask;
use crate::download::mirror::transform_url;
use crate::error::{MiaoError, Result};

use super::install::{JavaInstallPlan, install_dir_for};

/// The all-JREs manifest URL. The hash in the path is fixed per Mojang's launcher and
/// only changes when they roll out a new JRE component layout (rare).
const JAVA_RUNTIME_MANIFEST_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// All Java major versions Mojang ships JREs for. Verified against
/// `piston-meta.mojang.com/.../all.json` and the corresponding BMCLAPI mirror.
const SUPPORTED_MAJORS: &[u32] = &[8, 16, 17, 21, 25];

#[derive(Debug, Deserialize)]
struct ManifestRoot(HashMap<String, HashMap<String, Vec<JreComponent>>>);

#[derive(Debug, Deserialize)]
struct JreComponent {
    manifest: ManifestRef,
    version: ComponentVersion,
}

#[derive(Debug, Deserialize)]
struct ManifestRef {
    #[allow(dead_code)]
    sha1: String,
    #[allow(dead_code)]
    size: u64,
    url: String,
}

#[derive(Debug, Deserialize)]
struct ComponentVersion {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ComponentManifest {
    files: HashMap<String, FileEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum FileEntry {
    Directory,
    File {
        downloads: FileDownloads,
        executable: Option<bool>,
    },
    Link {
        target: String,
    },
}

#[derive(Debug, Deserialize)]
struct FileDownloads {
    raw: DownloadEntry,
    /// LZMA-compressed copy. We always download `raw` because BMCLAPI doesn't
    /// always host the lzma variant and decompression complicates retry/verification.
    #[serde(default)]
    #[allow(dead_code)]
    lzma: Option<DownloadEntry>,
}

#[derive(Debug, Deserialize)]
struct DownloadEntry {
    sha1: String,
    size: u64,
    url: String,
}

/// Returns the Mojang JRE component name (e.g. `java-runtime-gamma`) for the given
/// Java major version, or `None` if Mojang doesn't ship a JRE for that major.
///
/// The mapping uses the canonical (non-snapshot) component for each major. Mojang
/// also publishes `java-runtime-beta` (Java 17, predecessor of `gamma`) and
/// `java-runtime-gamma-snapshot`, both of which we intentionally skip in favour of the
/// stable equivalent.
pub fn component_for_major(major: u32) -> Option<&'static str> {
    match major {
        8 => Some("jre-legacy"),
        16 => Some("java-runtime-alpha"),
        17 => Some("java-runtime-gamma"),
        21 => Some("java-runtime-delta"),
        25 => Some("java-runtime-epsilon"),
        _ => None,
    }
}

/// Pick the smallest component that satisfies `required_major`. Used when a Minecraft
/// version requests a major (e.g. 11, 18, 19) that Mojang doesn't directly ship — Java
/// is binary-compatible upwards, so a newer LTS is always safe.
fn satisfying_component(required_major: u32) -> Option<(u32, &'static str)> {
    SUPPORTED_MAJORS
        .iter()
        .copied()
        .filter(|&m| m >= required_major)
        .min()
        .and_then(|m| component_for_major(m).map(|c| (m, c)))
}

/// Returns the Mojang platform key (`windows-x64`, `linux`, `mac-os`, etc.) for the
/// current host.
fn platform_key() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("windows-x64"),
        ("windows", "x86") => Some("windows-x86"),
        ("windows", "aarch64") => Some("windows-arm64"),
        ("linux", "x86_64") => Some("linux"),
        ("linux", "x86") => Some("linux-i386"),
        ("macos", "x86_64") => Some("mac-os"),
        ("macos", "aarch64") => Some("mac-os-arm64"),
        _ => None,
    }
}

/// Plan an install from the Mojang source. Performs network I/O for the two manifest
/// fetches but does not download any JRE files. Called by
/// [`crate::java::install::plan`] when [`JavaSource::Mojang`] is selected.
pub async fn plan_install(
    http: &reqwest::Client,
    config: &LauncherConfig,
    major_version: u32,
) -> Result<JavaInstallPlan> {
    let (effective_major, component) = satisfying_component(major_version).ok_or_else(|| {
        MiaoError::Other(format!(
            "Java {} is not available from Mojang. Supported versions: {:?}",
            major_version, SUPPORTED_MAJORS
        ))
    })?;

    let platform = platform_key().ok_or_else(|| {
        MiaoError::Other(format!(
            "Unsupported platform: {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;

    let mirror = &config.download_mirror;

    // 1. Fetch all.json.
    let all_url = transform_url(JAVA_RUNTIME_MANIFEST_URL, mirror);
    let all_json: ManifestRoot = fetch_json(http, &all_url)
        .await
        .map_err(|e| MiaoError::Other(format!("Failed to fetch JRE list ({}): {}", all_url, e)))?;

    let platform_components = all_json.0.get(platform).ok_or_else(|| {
        MiaoError::Other(format!(
            "Mojang doesn't publish a JRE for platform '{}'",
            platform
        ))
    })?;

    let runtimes = platform_components.get(component).ok_or_else(|| {
        MiaoError::Other(format!(
            "Mojang doesn't ship Java {} ('{}') for {}",
            effective_major, component, platform
        ))
    })?;

    let runtime = runtimes.first().ok_or_else(|| {
        MiaoError::Other(format!(
            "Mojang reports no builds of '{}' for {}",
            component, platform
        ))
    })?;

    // 2. Fetch component manifest (per-file index).
    let manifest_url = transform_url(&runtime.manifest.url, mirror);
    let manifest: ComponentManifest = fetch_json(http, &manifest_url).await.map_err(|e| {
        MiaoError::Other(format!(
            "Failed to fetch JRE component manifest ({}): {}",
            manifest_url, e
        ))
    })?;

    let install_dir = install_dir_for(config, JavaSource::Mojang, component);
    let mut tasks = Vec::with_capacity(manifest.files.len());
    let mut links = Vec::new();
    #[cfg(unix)]
    let mut executables = Vec::new();
    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;

    for (rel_path, entry) in manifest.files {
        let dest = install_dir.join(&rel_path);
        match entry {
            FileEntry::Directory => {
                // Created lazily by the file writer; nothing to track.
            }
            FileEntry::File {
                downloads,
                executable,
            } => {
                tasks.push(DownloadTask {
                    url: downloads.raw.url,
                    dest: dest.clone(),
                    sha1: Some(downloads.raw.sha1),
                    sha256: None,
                    size: Some(downloads.raw.size),
                });
                total_size += downloads.raw.size;
                file_count += 1;

                #[cfg(unix)]
                if executable.unwrap_or(false) {
                    executables.push(dest);
                }
                #[cfg(not(unix))]
                let _ = executable;
            }
            FileEntry::Link { target } => {
                links.push((dest, target));
            }
        }
    }

    Ok(JavaInstallPlan {
        variant: component.to_string(),
        version: runtime.version.name.clone(),
        major: effective_major,
        source_name: JavaSource::Mojang.display_name(),
        total_size,
        file_count,
        install_dir,
        tasks,
        archives: Vec::new(),
        links,
        #[cfg(unix)]
        executables,
    })
}

async fn fetch_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T> {
    let bytes = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    serde_json::from_slice(&bytes).map_err(MiaoError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_mapping_supports_minecraft_majors() {
        assert_eq!(component_for_major(8), Some("jre-legacy"));
        assert_eq!(component_for_major(16), Some("java-runtime-alpha"));
        assert_eq!(component_for_major(17), Some("java-runtime-gamma"));
        assert_eq!(component_for_major(21), Some("java-runtime-delta"));
        assert_eq!(component_for_major(25), Some("java-runtime-epsilon"));
    }

    #[test]
    fn component_mapping_returns_none_for_unsupported() {
        assert_eq!(component_for_major(11), None);
        assert_eq!(component_for_major(22), None);
        assert_eq!(component_for_major(0), None);
    }

    #[test]
    fn satisfying_component_picks_smallest_compatible() {
        assert_eq!(satisfying_component(8), Some((8, "jre-legacy")));
        assert_eq!(satisfying_component(17), Some((17, "java-runtime-gamma")));
        assert_eq!(satisfying_component(11), Some((16, "java-runtime-alpha")));
        assert_eq!(satisfying_component(18), Some((21, "java-runtime-delta")));
        assert_eq!(satisfying_component(20), Some((21, "java-runtime-delta")));
        assert_eq!(satisfying_component(24), Some((25, "java-runtime-epsilon")));
        assert_eq!(satisfying_component(99), None);
    }

    #[test]
    fn supported_majors_match_component_mapping() {
        for &major in SUPPORTED_MAJORS {
            assert!(
                component_for_major(major).is_some(),
                "major {} listed as supported but has no component",
                major
            );
        }
    }

    #[test]
    fn parses_manifest_root() {
        let json = r#"{
            "windows-x64": {
                "java-runtime-gamma": [{
                    "manifest": {
                        "sha1": "abc",
                        "size": 1234,
                        "url": "https://piston-meta.mojang.com/x.json"
                    },
                    "version": { "name": "17.0.8", "released": "2024-01-01" },
                    "availability": { "group": 1, "progress": 100 }
                }],
                "jre-legacy": []
            }
        }"#;
        let parsed: ManifestRoot = serde_json::from_str(json).unwrap();
        let win = parsed.0.get("windows-x64").unwrap();
        let gamma = win.get("java-runtime-gamma").unwrap();
        assert_eq!(gamma.len(), 1);
        assert_eq!(gamma[0].version.name, "17.0.8");
    }

    #[test]
    fn parses_component_manifest_files() {
        let json = r#"{
            "files": {
                "bin/java": {
                    "type": "file",
                    "executable": true,
                    "downloads": {
                        "raw": {
                            "sha1": "deadbeef",
                            "size": 100,
                            "url": "https://piston-data.mojang.com/bin/java"
                        }
                    }
                },
                "lib": { "type": "directory" },
                "lib/jli/libjli.dylib": {
                    "type": "link",
                    "target": "../libjli.dylib"
                }
            }
        }"#;
        let manifest: ComponentManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.files.len(), 3);

        match manifest.files.get("bin/java").unwrap() {
            FileEntry::File {
                downloads,
                executable,
            } => {
                assert_eq!(executable, &Some(true));
                assert_eq!(downloads.raw.size, 100);
            }
            _ => panic!("expected file"),
        }

        match manifest.files.get("lib").unwrap() {
            FileEntry::Directory => {}
            _ => panic!("expected directory"),
        }

        match manifest.files.get("lib/jli/libjli.dylib").unwrap() {
            FileEntry::Link { target } => assert_eq!(target, "../libjli.dylib"),
            _ => panic!("expected link"),
        }
    }
}
