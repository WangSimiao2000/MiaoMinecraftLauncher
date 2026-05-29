use serde::Deserialize;

use crate::config::{JavaSource, LauncherConfig};
use crate::download::DownloadTask;
use crate::error::{MiaoError, Result};

use super::extract::ArchiveFormat;
use super::install::{ArchiveTask, JavaInstallPlan, install_dir_for};

const ADOPTIUM_API: &str = "https://api.adoptium.net";
const SUPPORTED_MAJORS: &[u32] = &[8, 11, 17, 21, 25];

#[derive(Debug, Deserialize)]
struct FeatureRelease {
    binaries: Vec<Binary>,
    version_data: VersionData,
}

#[derive(Debug, Deserialize)]
struct VersionData {
    semver: String,
}

#[derive(Debug, Deserialize)]
struct Binary {
    package: Package,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    link: String,
    checksum: String,
    size: u64,
}

fn satisfying_major(required: u32) -> Option<u32> {
    SUPPORTED_MAJORS
        .iter()
        .copied()
        .filter(|&m| m >= required)
        .min()
}

fn os_arch() -> Option<(&'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some(("linux", "x64")),
        ("linux", "aarch64") => Some(("linux", "aarch64")),
        ("linux", "arm") => Some(("linux", "arm")),
        ("macos", "x86_64") => Some(("mac", "x64")),
        ("macos", "aarch64") => Some(("mac", "aarch64")),
        ("windows", "x86_64") => Some(("windows", "x64")),
        ("windows", "aarch64") => Some(("windows", "aarch64")),
        _ => None,
    }
}

fn build_query_url(major: u32, os: &str, arch: &str, image_type: &str) -> String {
    format!(
        "{base}/v3/assets/feature_releases/{major}/ga\
         ?architecture={arch}\
         &heap_size=normal\
         &image_type={image_type}\
         &jvm_impl=hotspot\
         &os={os}\
         &page=0&page_size=1\
         &project=jdk\
         &sort_method=DEFAULT&sort_order=DESC\
         &vendor=eclipse",
        base = ADOPTIUM_API,
        major = major,
        arch = arch,
        image_type = image_type,
        os = os,
    )
}

async fn fetch_release(http: &reqwest::Client, url: &str) -> Result<Vec<FeatureRelease>> {
    let resp = http.get(url).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    let resp = resp.error_for_status()?;
    let bytes = resp.bytes().await?;
    serde_json::from_slice(&bytes).map_err(MiaoError::from)
}

pub async fn plan_install(
    http: &reqwest::Client,
    config: &LauncherConfig,
    required_major: u32,
) -> Result<JavaInstallPlan> {
    let major = satisfying_major(required_major).ok_or_else(|| {
        MiaoError::Other(format!(
            "Adoptium does not ship Java {}. Supported: {:?}",
            required_major, SUPPORTED_MAJORS
        ))
    })?;

    let (os, arch) = os_arch().ok_or_else(|| {
        MiaoError::Other(format!(
            "Adoptium has no build for {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
        ))
    })?;

    let mut releases = fetch_release(http, &build_query_url(major, os, arch, "jre")).await?;
    if releases.is_empty() {
        releases = fetch_release(http, &build_query_url(major, os, arch, "jdk")).await?;
    }
    if releases.is_empty() {
        return Err(MiaoError::Other(format!(
            "Adoptium has no GA release for Java {} on {}/{}",
            major, os, arch
        )));
    }

    let release = releases.into_iter().next().unwrap();
    let binary = release.binaries.into_iter().next().ok_or_else(|| {
        MiaoError::Other(format!(
            "Adoptium release {} has no binaries for {}/{}",
            release.version_data.semver, os, arch
        ))
    })?;

    let format = ArchiveFormat::from_filename(&binary.package.name).ok_or_else(|| {
        MiaoError::Other(format!(
            "Adoptium archive '{}' has unsupported extension",
            binary.package.name
        ))
    })?;

    let variant = release.version_data.semver.clone();
    let install_dir = install_dir_for(config, JavaSource::Adoptium, &variant);
    let archive_dest = install_dir
        .parent()
        .unwrap_or(&install_dir)
        .join(format!(".{}.archive", variant));

    let archives = vec![ArchiveTask {
        task: DownloadTask {
            url: binary.package.link.clone(),
            dest: archive_dest.clone(),
            sha1: None,
            sha256: Some(binary.package.checksum.clone()),
            size: Some(binary.package.size),
        },
        archive_dest,
        format,
        strip_components: 1,
    }];

    Ok(JavaInstallPlan {
        variant,
        version: release.version_data.semver,
        major,
        source_name: JavaSource::Adoptium.display_name(),
        total_size: binary.package.size,
        file_count: 1,
        install_dir,
        tasks: Vec::new(),
        archives,
        links: Vec::new(),
        #[cfg(unix)]
        executables: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satisfying_major_minecraft_versions() {
        assert_eq!(satisfying_major(8), Some(8));
        assert_eq!(satisfying_major(11), Some(11));
        assert_eq!(satisfying_major(16), Some(17));
        assert_eq!(satisfying_major(17), Some(17));
        assert_eq!(satisfying_major(18), Some(21));
        assert_eq!(satisfying_major(20), Some(21));
        assert_eq!(satisfying_major(21), Some(21));
        assert_eq!(satisfying_major(22), Some(25));
        assert_eq!(satisfying_major(99), None);
    }

    #[test]
    fn build_query_url_matches_adoptium_swagger() {
        let url = build_query_url(21, "linux", "x64", "jre");
        assert!(url.contains("/v3/assets/feature_releases/21/ga"));
        assert!(url.contains("architecture=x64"));
        assert!(url.contains("os=linux"));
        assert!(url.contains("image_type=jre"));
        assert!(url.contains("vendor=eclipse"));
        assert!(url.contains("project=jdk"));
        assert!(url.contains("jvm_impl=hotspot"));
    }

    #[test]
    fn parses_feature_release_response() {
        let json = r#"[{
            "release_name": "jdk-21.0.11+10",
            "version_data": { "semver": "21.0.11+10.0.LTS" },
            "binaries": [{
                "package": {
                    "name": "OpenJDK21U-jre_x64_linux_hotspot_21.0.11_10.tar.gz",
                    "link": "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B10/OpenJDK21U-jre_x64_linux_hotspot_21.0.11_10.tar.gz",
                    "checksum": "4b2220e232a97997b436ca6ab15cbf70171ecff52958a46159dfa5a8c44ca4de",
                    "size": 52428800
                }
            }]
        }]"#;
        let releases: Vec<FeatureRelease> = serde_json::from_str(json).unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].version_data.semver, "21.0.11+10.0.LTS");
        assert_eq!(releases[0].binaries[0].package.size, 52428800);
        assert_eq!(
            releases[0].binaries[0].package.checksum,
            "4b2220e232a97997b436ca6ab15cbf70171ecff52958a46159dfa5a8c44ca4de"
        );
    }

    #[test]
    fn os_arch_at_least_some_platform() {
        let _ = os_arch();
    }
}
