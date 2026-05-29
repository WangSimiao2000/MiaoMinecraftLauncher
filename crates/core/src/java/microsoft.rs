use crate::config::{JavaSource, LauncherConfig};
use crate::download::DownloadTask;
use crate::error::{MiaoError, Result};

use super::extract::ArchiveFormat;
use super::install::{ArchiveTask, JavaInstallPlan, install_dir_for};

const AKA_MS_BASE: &str = "https://aka.ms/download-jdk";
const SUPPORTED_MAJORS: &[u32] = &[11, 17, 21, 25];

fn satisfying_major(required: u32) -> Option<u32> {
    SUPPORTED_MAJORS
        .iter()
        .copied()
        .filter(|&m| m >= required)
        .min()
}

fn os_arch_ext() -> Option<(&'static str, &'static str, &'static str)> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Some(("linux", "x64", "tar.gz")),
        ("linux", "aarch64") => Some(("linux", "aarch64", "tar.gz")),
        ("macos", "x86_64") => Some(("macOS", "x64", "tar.gz")),
        ("macos", "aarch64") => Some(("macOS", "aarch64", "tar.gz")),
        ("windows", "x86_64") => Some(("windows", "x64", "zip")),
        ("windows", "aarch64") => Some(("windows", "aarch64", "zip")),
        _ => None,
    }
}

fn build_archive_url(major: u32, os: &str, arch: &str, ext: &str) -> String {
    format!(
        "{base}/microsoft-jdk-{major}-{os}-{arch}.{ext}",
        base = AKA_MS_BASE,
        major = major,
        os = os,
        arch = arch,
        ext = ext,
    )
}

fn parse_sha256_file(content: &str) -> Option<(String, String)> {
    let line = content.lines().next()?;
    let mut parts = line.split_whitespace();
    let hash = parts.next()?.to_string();
    let filename = parts.next()?.to_string();
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some((hash, filename))
}

fn parse_version_from_filename(name: &str) -> Option<String> {
    let stripped = name.strip_prefix("microsoft-jdk-")?;
    let dash = stripped.find('-')?;
    Some(stripped[..dash].to_string())
}

pub async fn plan_install(
    http: &reqwest::Client,
    config: &LauncherConfig,
    required_major: u32,
) -> Result<JavaInstallPlan> {
    let major = satisfying_major(required_major).ok_or_else(|| {
        MiaoError::Other(format!(
            "Microsoft Build of OpenJDK does not ship Java {}. \
             Supported: {:?} (use Adoptium for Java 8).",
            required_major, SUPPORTED_MAJORS
        ))
    })?;

    let (os, arch, ext) = os_arch_ext().ok_or_else(|| {
        MiaoError::Other(format!(
            "Microsoft Build of OpenJDK has no release for {}/{}",
            std::env::consts::OS,
            std::env::consts::ARCH,
        ))
    })?;

    let archive_url = build_archive_url(major, os, arch, ext);
    let checksum_url = format!("{}.sha256sum.txt", archive_url);

    let checksum_body = http
        .get(&checksum_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let (sha256, filename) = parse_sha256_file(&checksum_body).ok_or_else(|| {
        MiaoError::Other(format!(
            "Microsoft sha256 file at {} is malformed: {:?}",
            checksum_url,
            checksum_body.lines().next().unwrap_or_default()
        ))
    })?;

    let version =
        parse_version_from_filename(&filename).unwrap_or_else(|| format!("{}.0.0", major));

    let format = ArchiveFormat::from_filename(&filename).ok_or_else(|| {
        MiaoError::Other(format!(
            "Microsoft archive '{}' has unsupported extension",
            filename
        ))
    })?;

    let install_dir = install_dir_for(config, JavaSource::Microsoft, &version);
    let archive_dest = install_dir
        .parent()
        .unwrap_or(&install_dir)
        .join(format!(".{}.archive", version));

    let archives = vec![ArchiveTask {
        task: DownloadTask {
            url: archive_url,
            dest: archive_dest.clone(),
            sha1: None,
            sha256: Some(sha256),
            size: None,
        },
        archive_dest,
        format,
        strip_components: 1,
    }];

    Ok(JavaInstallPlan {
        variant: version.clone(),
        version,
        major,
        source_name: JavaSource::Microsoft.display_name(),
        total_size: 0,
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
    fn satisfying_major_no_java_8() {
        assert_eq!(satisfying_major(8), Some(11));
        assert_eq!(satisfying_major(11), Some(11));
        assert_eq!(satisfying_major(17), Some(17));
        assert_eq!(satisfying_major(20), Some(21));
        assert_eq!(satisfying_major(21), Some(21));
        assert_eq!(satisfying_major(99), None);
    }

    #[test]
    fn url_template_linux_x64() {
        let url = build_archive_url(21, "linux", "x64", "tar.gz");
        assert_eq!(
            url,
            "https://aka.ms/download-jdk/microsoft-jdk-21-linux-x64.tar.gz"
        );
    }

    #[test]
    fn url_template_windows() {
        let url = build_archive_url(17, "windows", "x64", "zip");
        assert_eq!(
            url,
            "https://aka.ms/download-jdk/microsoft-jdk-17-windows-x64.zip"
        );
    }

    #[test]
    fn url_template_macos_camelcase() {
        let url = build_archive_url(21, "macOS", "aarch64", "tar.gz");
        assert!(url.contains("macOS-aarch64"));
    }

    #[test]
    fn parse_valid_sha256_line() {
        let body = "1d58b1335d019bfe1c7c56979c927d0f51222c6dd41c33772d8396ea6cd409c1  microsoft-jdk-21.0.11-linux-x64.tar.gz\n";
        let (hash, name) = parse_sha256_file(body).unwrap();
        assert_eq!(
            hash,
            "1d58b1335d019bfe1c7c56979c927d0f51222c6dd41c33772d8396ea6cd409c1"
        );
        assert_eq!(name, "microsoft-jdk-21.0.11-linux-x64.tar.gz");
    }

    #[test]
    fn parse_rejects_short_hash() {
        let body = "deadbeef  short.tar.gz\n";
        assert!(parse_sha256_file(body).is_none());
    }

    #[test]
    fn parse_rejects_non_hex() {
        let body =
            "ZZZZb1335d019bfe1c7c56979c927d0f51222c6dd41c33772d8396ea6cd409c1  file.tar.gz\n";
        assert!(parse_sha256_file(body).is_none());
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse_sha256_file("").is_none());
    }

    #[test]
    fn extract_version_from_filename() {
        assert_eq!(
            parse_version_from_filename("microsoft-jdk-21.0.11-linux-x64.tar.gz"),
            Some("21.0.11".to_string())
        );
        assert_eq!(
            parse_version_from_filename("microsoft-jdk-17.0.19-windows-x64.zip"),
            Some("17.0.19".to_string())
        );
    }

    #[test]
    fn extract_version_invalid_filename() {
        assert_eq!(parse_version_from_filename("not-a-jdk-archive"), None);
        assert_eq!(parse_version_from_filename("microsoft-jdk-noversion"), None);
    }
}
