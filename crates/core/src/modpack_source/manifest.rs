use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

const SUPPORTED_NAMESPACE: &str = "miao";
const SUPPORTED_MAJOR: u32 = 1;

const SOURCE_ID_RE: &str = r"^[a-z0-9-]{2,32}$";
const PACK_ID_RE: &str = r"^[a-z0-9.-]{2,64}$";
const MC_VERSION_RE: &str = r"^\d+\.\d+(\.\d+)?$";
const PACK_FORMAT_RE: &str = r"^[a-z0-9-]+:\d+\.\d+\.\d+$";

const FALLBACK_HOSTS: &[&str] = &["raw.githubusercontent.com", "cdn.jsdelivr.net"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub pack_format: String,
    pub source_id: String,
    pub source_name: String,
    pub author: String,
    #[serde(default)]
    pub homepage: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub allowed_hosts: Option<Vec<String>>,
    pub packs: Vec<ManifestPackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestPackEntry {
    pub id: String,
    pub display_name: String,
    pub summary: String,
    pub mc_versions: Vec<String>,
    pub loader: Loader,
    pub support_level: SupportLevel,
    pub pack_url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Fabric,
    Forge,
    Neoforge,
    Quilt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum SupportLevel {
    Active,
    Maintenance,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pack {
    pub pack_format: String,
    pub id: String,
    pub display_name: String,
    pub version: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub loader: Loader,
    pub mc_versions: Vec<String>,
    #[serde(default)]
    pub loader_versions: HashMap<String, String>,
    pub mods: Vec<PackMod>,
    #[serde(default)]
    pub config_overlay: Option<ConfigOverlay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackMod {
    pub source: ModSource,
    pub id: String,
    pub name: String,
    pub policy: ModPolicy,
    #[serde(default)]
    pub locked_version: Option<String>,
    pub criticality: Criticality,
    #[serde(default)]
    pub deprecated_after: Option<String>,
    #[serde(default)]
    pub replacement: Option<ReplacementRef>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Modrinth,
    Curseforge,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ModPolicy {
    Auto,
    Lock,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Criticality {
    Core,
    Optional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplacementRef {
    pub source: ModSource,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOverlay {
    pub base_url: String,
    pub files: Vec<OverlayFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayFile {
    pub path: String,
    pub target: String,
    pub sha256: String,
    #[serde(default)]
    pub applies_to_mc: Option<Vec<String>>,
    #[serde(default)]
    pub original_size: Option<u64>,
    #[serde(default)]
    pub preserve: bool,
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("malformed JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error(
        "pack_format must match {} (e.g. miao:1.0.0); got '{got}'",
        PACK_FORMAT_RE
    )]
    PackFormatMalformed { got: String },

    #[error(
        "pack_format namespace '{namespace}' is not supported; this MMCL only handles '{}:'",
        SUPPORTED_NAMESPACE
    )]
    PackFormatUnsupportedNamespace { namespace: String },

    #[error(
        "pack_format major version {major} is newer than this MMCL supports (max {max}); please upgrade MMCL"
    )]
    PackFormatTooNew { major: u32, max: u32 },

    #[error("source_id '{got}' must match {}", SOURCE_ID_RE)]
    SourceIdMalformed { got: String },

    #[error("pack id '{got}' must match {}", PACK_ID_RE)]
    PackIdMalformed { got: String },

    #[error("mc_versions must contain at least one entry")]
    NoMcVersions,

    #[error("mc_version '{got}' must match {}", MC_VERSION_RE)]
    McVersionMalformed { got: String },

    #[error("pack_url '{url}' is not allowed: {reason}")]
    PackUrlForbidden { url: String, reason: String },

    #[error("locked_version is required when policy = lock for mod '{mod_id}'")]
    LockedVersionMissing { mod_id: String },
}

pub fn parse_manifest(raw: &[u8], manifest_url: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = serde_json::from_slice(raw)?;
    validate_manifest(&manifest, manifest_url)?;
    Ok(manifest)
}

pub fn parse_pack(raw: &[u8]) -> Result<Pack, ManifestError> {
    let pack: Pack = serde_json::from_slice(raw)?;
    validate_pack(&pack)?;
    Ok(pack)
}

fn validate_manifest(m: &Manifest, manifest_url: &str) -> Result<(), ManifestError> {
    validate_pack_format(&m.pack_format)?;
    validate_source_id(&m.source_id)?;

    let host_policy = HostPolicy::from_manifest(m, manifest_url);
    for pack in &m.packs {
        validate_pack_id(&pack.id)?;
        validate_mc_versions(&pack.mc_versions)?;
        host_policy.check(&pack.pack_url)?;
    }
    Ok(())
}

fn validate_pack(p: &Pack) -> Result<(), ManifestError> {
    validate_pack_format(&p.pack_format)?;
    validate_pack_id(&p.id)?;
    validate_mc_versions(&p.mc_versions)?;
    for m in &p.mods {
        if m.policy == ModPolicy::Lock && m.locked_version.is_none() {
            return Err(ManifestError::LockedVersionMissing {
                mod_id: m.id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_pack_format(s: &str) -> Result<(), ManifestError> {
    let re = regex::Regex::new(PACK_FORMAT_RE).expect("static regex");
    if !re.is_match(s) {
        return Err(ManifestError::PackFormatMalformed { got: s.to_string() });
    }
    let (namespace, version) = s.split_once(':').expect("regex matched, colon present");
    if namespace != SUPPORTED_NAMESPACE {
        return Err(ManifestError::PackFormatUnsupportedNamespace {
            namespace: namespace.to_string(),
        });
    }
    let major: u32 = version
        .split('.')
        .next()
        .expect("regex matched, dot present")
        .parse()
        .expect("regex matched, digits");
    if major > SUPPORTED_MAJOR {
        return Err(ManifestError::PackFormatTooNew {
            major,
            max: SUPPORTED_MAJOR,
        });
    }
    Ok(())
}

fn validate_source_id(s: &str) -> Result<(), ManifestError> {
    let re = regex::Regex::new(SOURCE_ID_RE).expect("static regex");
    if !re.is_match(s) {
        Err(ManifestError::SourceIdMalformed { got: s.to_string() })
    } else {
        Ok(())
    }
}

fn validate_pack_id(s: &str) -> Result<(), ManifestError> {
    let re = regex::Regex::new(PACK_ID_RE).expect("static regex");
    if !re.is_match(s) {
        Err(ManifestError::PackIdMalformed { got: s.to_string() })
    } else {
        Ok(())
    }
}

fn validate_mc_versions(versions: &[String]) -> Result<(), ManifestError> {
    if versions.is_empty() {
        return Err(ManifestError::NoMcVersions);
    }
    let re = regex::Regex::new(MC_VERSION_RE).expect("static regex");
    for v in versions {
        if !re.is_match(v) {
            return Err(ManifestError::McVersionMalformed { got: v.clone() });
        }
    }
    Ok(())
}

struct HostPolicy<'a> {
    manifest_origin: Option<String>,
    allowed_hosts: &'a [String],
    use_fallback: bool,
}

impl<'a> HostPolicy<'a> {
    fn from_manifest(m: &'a Manifest, manifest_url: &str) -> Self {
        let manifest_origin = Url::parse(manifest_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string));

        let (allowed_hosts, use_fallback) = match &m.allowed_hosts {
            Some(hosts) => (hosts.as_slice(), false),
            None => ([].as_slice(), true),
        };
        HostPolicy {
            manifest_origin,
            allowed_hosts,
            use_fallback,
        }
    }

    fn check(&self, url_str: &str) -> Result<(), ManifestError> {
        let url = Url::parse(url_str).map_err(|_| ManifestError::PackUrlForbidden {
            url: url_str.to_string(),
            reason: "not a valid URL".to_string(),
        })?;
        if url.scheme() != "https" {
            return Err(ManifestError::PackUrlForbidden {
                url: url_str.to_string(),
                reason: "scheme must be https".to_string(),
            });
        }
        let host = url
            .host_str()
            .ok_or_else(|| ManifestError::PackUrlForbidden {
                url: url_str.to_string(),
                reason: "missing host".to_string(),
            })?;

        if self
            .manifest_origin
            .as_deref()
            .is_some_and(|o| o.eq_ignore_ascii_case(host))
        {
            return Ok(());
        }

        if self
            .allowed_hosts
            .iter()
            .any(|h| h.eq_ignore_ascii_case(host))
        {
            return Ok(());
        }

        if self.use_fallback && FALLBACK_HOSTS.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            return Ok(());
        }

        Err(ManifestError::PackUrlForbidden {
            url: url_str.to_string(),
            reason: format!("host '{host}' is not in allowed_hosts or fallback whitelist"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MANIFEST_URL: &str = "https://example.com/manifest.json";

    fn manifest_full(extra: &str) -> String {
        format!(
            r#"{{
                "pack_format": "miao:1.0.0",
                "source_id": "miao",
                "source_name": "x",
                "author": "y",
                "updated_at": "2026-05-29T10:00:00Z",
                "packs": [{{
                    "id": "p1",
                    "display_name": "Pack 1",
                    "summary": "s",
                    "mc_versions": ["1.21.5"],
                    "loader": "fabric",
                    "support_level": "active",
                    "pack_url": "https://raw.githubusercontent.com/u/r/main/p.json"
                }}]
                {extra}
            }}"#
        )
    }

    #[test]
    fn t_schema_01_manifest_valid_loads() {
        let raw = manifest_full("");
        let m = parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect("valid manifest");
        assert_eq!(m.source_id, "miao");
        assert_eq!(m.packs.len(), 1);
        assert_eq!(m.packs[0].loader, Loader::Fabric);
    }

    #[test]
    fn t_schema_02_pack_format_major_too_high_rejected() {
        let raw = manifest_full("").replace("miao:1.0.0", "miao:99.0.0");
        let err = parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect_err("should reject");
        assert!(matches!(
            err,
            ManifestError::PackFormatTooNew { major: 99, .. }
        ));
    }

    #[test]
    fn t_schema_03_pack_format_minor_higher_accepted() {
        let raw = manifest_full("").replace("miao:1.0.0", "miao:1.99.0");
        parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect("minor higher should still parse");
    }

    #[test]
    fn t_schema_04_pack_format_unknown_namespace_rejected() {
        let raw = manifest_full("").replace("miao:1.0.0", "packwiz:1.0.0");
        let err = parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect_err("should reject");
        assert!(matches!(
            err,
            ManifestError::PackFormatUnsupportedNamespace { .. }
        ));
    }

    #[test]
    fn t_schema_05_source_id_invalid_chars_rejected() {
        let raw = manifest_full("").replace("\"miao\"", "\"Miao_2\"");
        let err = parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect_err("should reject");
        assert!(matches!(err, ManifestError::SourceIdMalformed { .. }));
    }

    #[test]
    fn t_schema_06_pack_url_not_in_allowed_hosts_rejected() {
        let extra = r#", "allowed_hosts": ["only-this.com"]"#;
        let raw = manifest_full(extra);
        let err = parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect_err("should reject");
        assert!(matches!(err, ManifestError::PackUrlForbidden { .. }));
    }

    #[test]
    fn t_schema_07_pack_url_in_allowed_hosts_accepted() {
        let extra = r#", "allowed_hosts": ["raw.githubusercontent.com"]"#;
        let raw = manifest_full(extra);
        parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect("allowed host should pass");
    }

    #[test]
    fn t_schema_08_pack_url_same_origin_always_allowed() {
        let extra = r#", "allowed_hosts": ["only-this.com"]"#;
        let raw = manifest_full(extra).replace(
            "https://raw.githubusercontent.com/u/r/main/p.json",
            "https://example.com/p.json",
        );
        parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect("same origin should pass");
    }

    #[test]
    fn t_schema_09_unknown_field_silently_dropped() {
        let extra = r#", "future_field": "ignored", "another": 42"#;
        let raw = manifest_full(extra);
        parse_manifest(raw.as_bytes(), TEST_MANIFEST_URL).expect("unknown fields are ignored");
    }

    #[test]
    fn t_schema_10_pack_with_deprecated_after_parses() {
        let pack_json = r#"{
            "pack_format": "miao:1.0.0",
            "id": "p1",
            "display_name": "Pack 1",
            "version": "0.1.0",
            "loader": "fabric",
            "mc_versions": ["1.21.5"],
            "mods": [{
                "source": "modrinth",
                "id": "AANobbMI",
                "name": "Sodium",
                "policy": "auto",
                "criticality": "core",
                "deprecated_after": "1.21.6",
                "replacement": {"source": "modrinth", "id": "newsodium"}
            }]
        }"#;
        let p = parse_pack(pack_json.as_bytes()).expect("parses");
        assert_eq!(p.mods[0].deprecated_after.as_deref(), Some("1.21.6"));
        assert!(p.mods[0].replacement.is_some());
    }

    #[test]
    fn t_schema_11_overlay_with_preserve_parses() {
        let pack_json = r#"{
            "pack_format": "miao:1.0.0",
            "id": "p1",
            "display_name": "Pack 1",
            "version": "0.1.0",
            "loader": "fabric",
            "mc_versions": ["1.21.5"],
            "mods": [],
            "config_overlay": {
                "base_url": "https://raw.githubusercontent.com/u/r/main/c",
                "files": [{
                    "path": "_common/options.txt",
                    "target": "options.txt",
                    "sha256": "deadbeef",
                    "preserve": true,
                    "original_size": 2048
                }]
            }
        }"#;
        let p = parse_pack(pack_json.as_bytes()).expect("parses");
        let overlay = p.config_overlay.expect("present");
        assert!(overlay.files[0].preserve);
        assert_eq!(overlay.files[0].original_size, Some(2048));
    }

    #[test]
    fn t_schema_12_real_fixture_parses() {
        let manifest_raw = include_bytes!("fixtures/miao_manifest.json");
        let pack_raw = include_bytes!("fixtures/miao_pack.json");

        let url =
            "https://raw.githubusercontent.com/WangSimiao2000/miao-modpacks/main/manifest.json";
        let m = parse_manifest(manifest_raw, url).expect("real fixture manifest parses");
        assert_eq!(m.source_id, "miao");
        assert_eq!(m.packs[0].id, "miao-1.21-base-test");

        let p = parse_pack(pack_raw).expect("real fixture pack parses");
        assert_eq!(p.id, "miao-1.21-base-test");
        assert_eq!(p.mods.len(), 3);
    }

    #[test]
    fn t_schema_13_lock_policy_without_locked_version_rejected() {
        let pack_json = r#"{
            "pack_format": "miao:1.0.0",
            "id": "p1",
            "display_name": "Pack 1",
            "version": "0.1.0",
            "loader": "fabric",
            "mc_versions": ["1.21.5"],
            "mods": [{
                "source": "modrinth",
                "id": "AANobbMI",
                "name": "Sodium",
                "policy": "lock",
                "criticality": "core"
            }]
        }"#;
        let err = parse_pack(pack_json.as_bytes()).expect_err("should reject");
        assert!(matches!(err, ManifestError::LockedVersionMissing { .. }));
    }
}
