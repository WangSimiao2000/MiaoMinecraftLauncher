use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub major_version: u32,
    pub arch: String,
}

impl JavaInstallation {
    pub fn is_64bit(&self) -> bool {
        matches!(self.arch.as_str(), "x86_64" | "aarch64")
    }
}

pub fn detect_system_java() -> Vec<JavaInstallation> {
    detect_java_in_paths(&["/usr/lib/jvm", "/usr/local/lib/jvm", "/usr/java"])
}

pub fn detect_java_in_paths(search_paths: &[&str]) -> Vec<JavaInstallation> {
    let mut installations = Vec::new();

    for base in search_paths {
        let base_path = PathBuf::from(base);
        if !base_path.exists() {
            continue;
        }

        if let Ok(entries) = std::fs::read_dir(&base_path) {
            for entry in entries.flatten() {
                let java_bin = entry.path().join("bin/java");
                if java_bin.exists()
                    && let Ok(info) = probe_java(&java_bin)
                {
                    installations.push(info);
                }
            }
        }
    }

    installations.sort_by_key(|j| j.major_version);
    installations
}

pub fn probe_java(java_bin: &PathBuf) -> Result<JavaInstallation> {
    let output = std::process::Command::new(java_bin)
        .arg("-version")
        .output()?;

    let version_output = String::from_utf8_lossy(&output.stderr);
    let version = parse_java_version(&version_output).unwrap_or_else(|| "unknown".to_string());
    let major = parse_major_version(&version);

    Ok(JavaInstallation {
        path: java_bin.clone(),
        version,
        major_version: major,
        arch: std::env::consts::ARCH.to_string(),
    })
}

pub fn parse_java_version(output: &str) -> Option<String> {
    let first_line = output.lines().next()?;
    let start = first_line.find('"')? + 1;
    let end = first_line[start..].find('"')? + start;
    Some(first_line[start..end].to_string())
}

pub fn parse_major_version(version: &str) -> u32 {
    if version == "unknown" {
        return 0;
    }
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

pub fn find_exact_java(
    installations: &[JavaInstallation],
    major: u32,
) -> Option<&JavaInstallation> {
    installations.iter().find(|j| j.major_version == major)
}

pub fn validate_java_path(path: &Path) -> bool {
    path.exists() && path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_installations() -> Vec<JavaInstallation> {
        vec![
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-8/bin/java"),
                version: "1.8.0_392".to_string(),
                major_version: 8,
                arch: "x86_64".to_string(),
            },
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-11/bin/java"),
                version: "11.0.21".to_string(),
                major_version: 11,
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
                arch: "aarch64".to_string(),
            },
        ]
    }

    #[test]
    fn parse_java_8_version() {
        let output = r#"openjdk version "1.8.0_392""#;
        assert_eq!(parse_java_version(output), Some("1.8.0_392".to_string()));
    }

    #[test]
    fn parse_java_11_version() {
        let output = r#"openjdk version "11.0.21" 2023-10-17"#;
        assert_eq!(parse_java_version(output), Some("11.0.21".to_string()));
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
    fn parse_java_graalvm_version() {
        let output = r#"openjdk version "21.0.1-graal" 2023-10-17"#;
        assert_eq!(parse_java_version(output), Some("21.0.1-graal".to_string()));
    }

    #[test]
    fn parse_java_version_no_quotes() {
        let output = "no version here";
        assert_eq!(parse_java_version(output), None);
    }

    #[test]
    fn parse_java_version_empty_input() {
        assert_eq!(parse_java_version(""), None);
    }

    #[test]
    fn parse_java_version_multiline() {
        let output = "openjdk version \"17.0.9\" 2023-10-17\nOpenJDK Runtime\nOpenJDK 64-Bit";
        assert_eq!(parse_java_version(output), Some("17.0.9".to_string()));
    }

    #[test]
    fn major_version_java_8() {
        assert_eq!(parse_major_version("1.8.0_392"), 8);
    }

    #[test]
    fn major_version_java_11() {
        assert_eq!(parse_major_version("11.0.21"), 11);
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
    fn major_version_graalvm() {
        assert_eq!(parse_major_version("21.0.1-graal"), 21);
    }

    #[test]
    fn major_version_unknown() {
        assert_eq!(parse_major_version("unknown"), 0);
    }

    #[test]
    fn major_version_single_number() {
        assert_eq!(parse_major_version("21"), 21);
    }

    #[test]
    fn find_compatible_selects_minimum_sufficient() {
        let installs = make_installations();
        let result = find_compatible_java(&installs, 17);
        assert_eq!(result.unwrap().major_version, 17);
    }

    #[test]
    fn find_compatible_returns_higher_when_exact_missing() {
        let installs = vec![
            JavaInstallation {
                path: PathBuf::from("/usr/lib/jvm/java-8/bin/java"),
                version: "1.8.0".to_string(),
                major_version: 8,
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
        assert_eq!(result.unwrap().major_version, 21);
    }

    #[test]
    fn find_compatible_returns_none_when_no_match() {
        let installs = make_installations();
        let result = find_compatible_java(&installs, 25);
        assert!(result.is_none());
    }

    #[test]
    fn find_compatible_empty_list() {
        let result = find_compatible_java(&[], 17);
        assert!(result.is_none());
    }

    #[test]
    fn find_exact_java_found() {
        let installs = make_installations();
        let result = find_exact_java(&installs, 17);
        assert_eq!(result.unwrap().major_version, 17);
        assert_eq!(result.unwrap().version, "17.0.9");
    }

    #[test]
    fn find_exact_java_not_found() {
        let installs = make_installations();
        let result = find_exact_java(&installs, 14);
        assert!(result.is_none());
    }

    #[test]
    fn java_installation_is_64bit() {
        let install_x86_64 = JavaInstallation {
            path: PathBuf::from("/bin/java"),
            version: "17.0.9".to_string(),
            major_version: 17,
            arch: "x86_64".to_string(),
        };
        assert!(install_x86_64.is_64bit());

        let install_aarch64 = JavaInstallation {
            path: PathBuf::from("/bin/java"),
            version: "17.0.9".to_string(),
            major_version: 17,
            arch: "aarch64".to_string(),
        };
        assert!(install_aarch64.is_64bit());

        let install_32 = JavaInstallation {
            path: PathBuf::from("/bin/java"),
            version: "17.0.9".to_string(),
            major_version: 17,
            arch: "i686".to_string(),
        };
        assert!(!install_32.is_64bit());
    }

    #[test]
    fn java_installation_serialization_roundtrip() {
        let install = JavaInstallation {
            path: PathBuf::from("/usr/lib/jvm/java-17/bin/java"),
            version: "17.0.9".to_string(),
            major_version: 17,
            arch: "x86_64".to_string(),
        };

        let json = serde_json::to_string(&install).unwrap();
        let deserialized: JavaInstallation = serde_json::from_str(&json).unwrap();
        assert_eq!(install, deserialized);
    }

    #[test]
    fn validate_java_path_existing_file() {
        let path = PathBuf::from("/usr/bin/env");
        assert!(validate_java_path(&path) || !path.exists());
    }

    #[test]
    fn validate_java_path_nonexistent() {
        let path = PathBuf::from("/nonexistent/path/to/java");
        assert!(!validate_java_path(&path));
    }

    #[test]
    fn validate_java_path_directory() {
        let path = PathBuf::from("/tmp");
        assert!(!validate_java_path(&path));
    }

    #[test]
    fn detect_java_in_nonexistent_paths() {
        let result = detect_java_in_paths(&["/nonexistent_path_xyz_123"]);
        assert!(result.is_empty());
    }

    #[test]
    fn detect_java_in_empty_paths() {
        let result = detect_java_in_paths(&[]);
        assert!(result.is_empty());
    }
}
