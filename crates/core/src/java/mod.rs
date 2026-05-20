use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub major_version: u32,
    pub arch: String,
}

pub fn detect_system_java() -> Vec<JavaInstallation> {
    let mut installations = Vec::new();

    let search_paths = [
        "/usr/lib/jvm",
        "/usr/local/lib/jvm",
        "/usr/java",
    ];

    for base in &search_paths {
        let base_path = PathBuf::from(base);
        if !base_path.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let java_bin = entry.path().join("bin/java");
                if java_bin.exists() {
                    if let Ok(info) = probe_java(&java_bin) {
                        installations.push(info);
                    }
                }
            }
        }
    }

    installations
}

fn probe_java(java_bin: &PathBuf) -> Result<JavaInstallation> {
    let output = std::process::Command::new(java_bin)
        .arg("-version")
        .output()?;

    let version_output = String::from_utf8_lossy(&output.stderr);
    let version = parse_java_version(&version_output)
        .unwrap_or_else(|| "unknown".to_string());
    let major = parse_major_version(&version);

    Ok(JavaInstallation {
        path: java_bin.clone(),
        version,
        major_version: major,
        arch: std::env::consts::ARCH.to_string(),
    })
}

fn parse_java_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    let start = first_line.find('"')? + 1;
    let end = first_line[start..].find('"')? + start;
    Some(first_line[start..end].to_string())
}

fn parse_major_version(version: &str) -> u32 {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.first() == Some(&"1") {
        parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(8)
    } else {
        parts
            .first()
            .and_then(|s| s.split('-').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(8)
    }
}

pub fn find_compatible_java(
    installations: &[JavaInstallation],
    required_major: u32,
) -> Option<&JavaInstallation> {
    installations
        .iter()
        .filter(|j| j.major_version >= required_major)
        .min_by_key(|j| j.major_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_java_8_version() {
        let output = r#"openjdk version "1.8.0_392""#;
        assert_eq!(parse_java_version(output), Some("1.8.0_392".to_string()));
    }

    #[test]
    fn parse_java_17_version() {
        let output = r#"openjdk version "17.0.9" 2023-10-17"#;
        assert_eq!(parse_java_version(output), Some("17.0.9".to_string()));
    }

    #[test]
    fn parse_java_21_version() {
        let output = r#"openjdk version "21.0.1" 2023-10-17 LTS"#;
        assert_eq!(parse_java_version(output), Some("21.0.1".to_string()));
    }

    #[test]
    fn major_version_java_8() {
        assert_eq!(parse_major_version("1.8.0_392"), 8);
    }

    #[test]
    fn major_version_java_17() {
        assert_eq!(parse_major_version("17.0.9"), 17);
    }

    #[test]
    fn major_version_java_21() {
        assert_eq!(parse_major_version("21.0.1"), 21);
    }

    #[test]
    fn find_compatible_selects_minimum_sufficient() {
        let installs = vec![
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-8/bin/java"),
                version: "1.8.0".to_string(),
                major_version: 8,
                arch: "x86_64".to_string(),
            },
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-17/bin/java"),
                version: "17.0.9".to_string(),
                major_version: 17,
                arch: "x86_64".to_string(),
            },
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-21/bin/java"),
                version: "21.0.1".to_string(),
                major_version: 21,
                arch: "x86_64".to_string(),
            },
        ];

        let result = find_compatible_java(&installs, 17);
        assert_eq!(result.unwrap().major_version, 17);
    }

    #[test]
    fn find_compatible_returns_none_when_no_match() {
        let installs = vec![JavaInstallation {
            path: PathBuf::from("/usr/lib/jvm/java-8/bin/java"),
            version: "1.8.0".to_string(),
            major_version: 8,
            arch: "x86_64".to_string(),
        }];

        let result = find_compatible_java(&installs, 17);
        assert!(result.is_none());
    }
}
