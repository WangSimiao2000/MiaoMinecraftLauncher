use std::sync::Arc;

use async_trait::async_trait;
use chrono::DateTime;

use crate::curseforge::api::{CfFile, CfMod, CurseForgeClient};
use crate::http::HttpClient;
use crate::modpack_source::manifest::{Loader, ModSource};
use crate::modpack_source::resolver::{
    CandidateVersion, DependencyEdge, DependencyKind, ProjectMeta, ReleaseType, ResolverDataSource,
};

const CF_RELATION_REQUIRED: u32 = 3;
const CF_RELATION_OPTIONAL: u32 = 2;
const CF_RELATION_INCOMPATIBLE: u32 = 5;
const CF_RELATION_EMBEDDED: u32 = 1;

const CF_RELEASE_RELEASE: u32 = 1;
const CF_RELEASE_BETA: u32 = 2;
const CF_RELEASE_ALPHA: u32 = 3;

pub struct LiveResolverDataSource<H: HttpClient + 'static> {
    pub modrinth: Arc<H>,
    pub curseforge: Option<Arc<CurseForgeClient>>,
}

impl<H: HttpClient + 'static> LiveResolverDataSource<H> {
    pub fn new(modrinth: Arc<H>, curseforge: Option<Arc<CurseForgeClient>>) -> Self {
        Self {
            modrinth,
            curseforge,
        }
    }
}

#[async_trait]
impl<H: HttpClient + 'static> ResolverDataSource for LiveResolverDataSource<H> {
    async fn fetch_project_meta(
        &self,
        source: ModSource,
        project_id: &str,
    ) -> Result<ProjectMeta, String> {
        match source {
            ModSource::Modrinth => fetch_modrinth_meta(self.modrinth.as_ref(), project_id).await,
            ModSource::Curseforge => {
                let cf = self
                    .curseforge
                    .as_ref()
                    .ok_or_else(|| "CurseForge client not configured".to_string())?;
                let mod_id: u32 = project_id
                    .parse()
                    .map_err(|_| format!("CF project_id must be u32, got '{project_id}'"))?;
                let cf_mod: CfMod = cf
                    .get_mod(mod_id)
                    .await
                    .map_err(|e| format!("CF get_mod {mod_id}: {e}"))?;
                let last = cf_mod
                    .latest_files_indexes
                    .first()
                    .map(|_| chrono::Utc::now());
                Ok(ProjectMeta {
                    project_id: project_id.to_string(),
                    display_name: cf_mod.name,
                    last_release: last,
                })
            }
        }
    }

    async fn fetch_compatible_versions(
        &self,
        source: ModSource,
        project_id: &str,
        mc_version: &str,
        loader: Loader,
    ) -> Result<Vec<CandidateVersion>, String> {
        match source {
            ModSource::Modrinth => {
                let versions = crate::modrinth::api::get_project_versions(
                    self.modrinth.as_ref(),
                    project_id,
                    Some(mc_version),
                    Some(loader.canonical()),
                )
                .await
                .map_err(|e| format!("Modrinth versions for {project_id}: {e}"))?;
                Ok(versions
                    .into_iter()
                    .filter_map(|v| modrinth_version_to_candidate(project_id, v))
                    .collect())
            }
            ModSource::Curseforge => {
                let cf = self
                    .curseforge
                    .as_ref()
                    .ok_or_else(|| "CurseForge client not configured".to_string())?;
                let mod_id: u32 = project_id
                    .parse()
                    .map_err(|_| format!("CF project_id must be u32, got '{project_id}'"))?;
                let files = cf
                    .get_mod_files(mod_id, Some(mc_version), Some(loader.canonical()))
                    .await
                    .map_err(|e| format!("CF get_mod_files {mod_id}: {e}"))?;
                let display_name = cf
                    .get_mod(mod_id)
                    .await
                    .map(|m| m.name)
                    .unwrap_or_else(|_| project_id.to_string());
                Ok(files
                    .into_iter()
                    .filter_map(|f| cf_file_to_candidate(project_id, &display_name, f))
                    .collect())
            }
        }
    }

    async fn fetch_locked_version(
        &self,
        source: ModSource,
        version_id: &str,
    ) -> Result<CandidateVersion, String> {
        match source {
            ModSource::Modrinth => {
                let url = format!("https://api.modrinth.com/v2/version/{version_id}");
                let v: crate::modrinth::api::ProjectVersion = self
                    .modrinth
                    .get_json(&url)
                    .await
                    .map_err(|e| format!("Modrinth version {version_id}: {e}"))?;
                modrinth_version_to_candidate(&v.project_id.clone(), v).ok_or_else(|| {
                    format!("Modrinth locked version {version_id} has no primary file")
                })
            }
            ModSource::Curseforge => Err(
                "CurseForge locked_version not yet supported in Phase 1; use policy=auto"
                    .to_string(),
            ),
        }
    }

    async fn fetch_loader_supported_mc_versions(
        &self,
        loader_version_id: &str,
    ) -> Result<Vec<String>, String> {
        let url = format!("https://api.modrinth.com/v2/version/{loader_version_id}");
        let v: crate::modrinth::api::ProjectVersion = self
            .modrinth
            .get_json(&url)
            .await
            .map_err(|e| format!("Modrinth loader version {loader_version_id}: {e}"))?;
        Ok(v.game_versions)
    }
}

async fn fetch_modrinth_meta<H: HttpClient>(
    http: &H,
    project_id: &str,
) -> Result<ProjectMeta, String> {
    #[derive(serde::Deserialize)]
    struct ProjectInfo {
        title: String,
        updated: String,
    }
    let url = format!("https://api.modrinth.com/v2/project/{project_id}");
    let info: ProjectInfo = http
        .get_json(&url)
        .await
        .map_err(|e| format!("Modrinth project {project_id}: {e}"))?;
    let last_release = DateTime::parse_from_rfc3339(&info.updated)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));
    Ok(ProjectMeta {
        project_id: project_id.to_string(),
        display_name: info.title,
        last_release,
    })
}

fn modrinth_version_to_candidate(
    fallback_project_id: &str,
    v: crate::modrinth::api::ProjectVersion,
) -> Option<CandidateVersion> {
    let primary_file = v
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| v.files.first())?;
    let date_published = DateTime::parse_from_rfc3339(&v.date_published)
        .ok()?
        .with_timezone(&chrono::Utc);
    let release_type = match v.version_type.as_str() {
        "release" => ReleaseType::Release,
        "beta" => ReleaseType::Beta,
        "alpha" => ReleaseType::Alpha,
        _ => ReleaseType::Release,
    };
    let dependencies = v
        .dependencies
        .iter()
        .filter_map(|d| {
            let pid = d.project_id.clone()?;
            let kind = match d.dependency_type.as_str() {
                "required" => DependencyKind::Required,
                "optional" => DependencyKind::Optional,
                "incompatible" => DependencyKind::Incompatible,
                "embedded" => DependencyKind::Embedded,
                _ => return None,
            };
            Some(DependencyEdge {
                project_id: pid,
                kind,
            })
        })
        .collect();

    let loader_dep_version_id = v
        .dependencies
        .iter()
        .find(|d| d.project_id.as_deref() == Some("P7dR8mSH"))
        .and_then(|d| d.version_id.clone());

    Some(CandidateVersion {
        version_id: v.id,
        project_id: if v.project_id.is_empty() {
            fallback_project_id.to_string()
        } else {
            v.project_id
        },
        display_name: v.name,
        mc_versions: v.game_versions,
        loaders: v.loaders,
        release_type,
        date_published,
        file_url: primary_file.url.clone(),
        file_name: primary_file.filename.clone(),
        file_size: primary_file.size,
        file_sha1: primary_file.hashes.sha1.clone(),
        file_sha512: primary_file.hashes.sha512.clone(),
        dependencies,
        loader_dep_version_id,
    })
}

fn cf_file_to_candidate(
    project_id: &str,
    display_name: &str,
    f: CfFile,
) -> Option<CandidateVersion> {
    let release_type = match f.release_type {
        CF_RELEASE_RELEASE => ReleaseType::Release,
        CF_RELEASE_BETA => ReleaseType::Beta,
        CF_RELEASE_ALPHA => ReleaseType::Alpha,
        _ => ReleaseType::Release,
    };
    let url = f.download_url?;
    let dependencies = f
        .dependencies
        .iter()
        .filter_map(|d| {
            let kind = match d.relation_type {
                CF_RELATION_REQUIRED => DependencyKind::Required,
                CF_RELATION_OPTIONAL => DependencyKind::Optional,
                CF_RELATION_INCOMPATIBLE => DependencyKind::Incompatible,
                CF_RELATION_EMBEDDED => DependencyKind::Embedded,
                _ => return None,
            };
            Some(DependencyEdge {
                project_id: d.mod_id.to_string(),
                kind,
            })
        })
        .collect();
    let sha1 = f
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.clone());

    Some(CandidateVersion {
        version_id: f.id.to_string(),
        project_id: project_id.to_string(),
        display_name: display_name.to_string(),
        mc_versions: f
            .game_versions
            .iter()
            .filter(|v| !v.is_empty() && v.chars().next().unwrap_or(' ').is_ascii_digit())
            .cloned()
            .collect(),
        loaders: f
            .game_versions
            .iter()
            .filter(|v| !v.is_empty() && !v.chars().next().unwrap_or(' ').is_ascii_digit())
            .map(|v| v.to_ascii_lowercase())
            .collect(),
        release_type,
        date_published: chrono::Utc::now(),
        file_url: url,
        file_name: f.file_name,
        file_size: f.file_length,
        file_sha1: sha1,
        file_sha512: None,
        dependencies,
        loader_dep_version_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modrinth::api::{Dependency, FileHashes, ProjectVersion, VersionFile};

    fn modrinth_version(deps: Vec<Dependency>) -> ProjectVersion {
        ProjectVersion {
            id: "vid1".to_string(),
            project_id: "proj1".to_string(),
            name: "Test Mod 1.0".to_string(),
            version_number: "1.0".to_string(),
            game_versions: vec!["1.21.5".to_string()],
            loaders: vec!["fabric".to_string()],
            files: vec![VersionFile {
                url: "https://cdn.example.com/test.jar".to_string(),
                filename: "test.jar".to_string(),
                primary: true,
                size: 1024,
                hashes: FileHashes {
                    sha1: Some("aaa".to_string()),
                    sha512: Some("bbb".to_string()),
                },
            }],
            dependencies: deps,
            version_type: "release".to_string(),
            downloads: 1,
            date_published: "2026-05-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn t_live_01_modrinth_version_required_dep_extracted() {
        let v = modrinth_version(vec![Dependency {
            project_id: Some("dep-x".to_string()),
            version_id: None,
            dependency_type: "required".to_string(),
        }]);
        let c = modrinth_version_to_candidate("proj1", v).unwrap();
        assert_eq!(c.dependencies.len(), 1);
        assert_eq!(c.dependencies[0].kind, DependencyKind::Required);
        assert_eq!(c.dependencies[0].project_id, "dep-x");
    }

    #[test]
    fn t_live_02_modrinth_version_dep_without_project_id_dropped() {
        let v = modrinth_version(vec![Dependency {
            project_id: None,
            version_id: Some("v-only".to_string()),
            dependency_type: "required".to_string(),
        }]);
        let c = modrinth_version_to_candidate("proj1", v).unwrap();
        assert!(c.dependencies.is_empty());
    }

    #[test]
    fn t_live_03_modrinth_loader_dep_version_id_captured() {
        let v = modrinth_version(vec![Dependency {
            project_id: Some("P7dR8mSH".to_string()),
            version_id: Some("loader-vid".to_string()),
            dependency_type: "required".to_string(),
        }]);
        let c = modrinth_version_to_candidate("proj1", v).unwrap();
        assert_eq!(c.loader_dep_version_id.as_deref(), Some("loader-vid"));
    }

    #[test]
    fn t_live_04_modrinth_no_primary_file_falls_back_to_first() {
        let mut v = modrinth_version(vec![]);
        v.files[0].primary = false;
        let c = modrinth_version_to_candidate("proj1", v).unwrap();
        assert_eq!(c.file_name, "test.jar");
    }

    #[test]
    fn t_live_05_modrinth_release_type_mapping() {
        let mut v = modrinth_version(vec![]);
        v.version_type = "beta".to_string();
        assert_eq!(
            modrinth_version_to_candidate("proj1", v)
                .unwrap()
                .release_type,
            ReleaseType::Beta
        );
    }

    #[test]
    fn t_live_06_cf_file_strips_loader_strings_from_game_versions() {
        let f = CfFile {
            id: 5000,
            mod_id: 12345,
            display_name: "Test".to_string(),
            file_name: "test.jar".to_string(),
            file_length: 2048,
            download_url: Some("https://edge.forgecdn.net/test.jar".to_string()),
            game_versions: vec![
                "1.21.5".to_string(),
                "Fabric".to_string(),
                "1.21.4".to_string(),
            ],
            dependencies: vec![],
            hashes: vec![crate::curseforge::api::CfHash {
                value: "sha1abc".to_string(),
                algo: 1,
            }],
            release_type: 1,
        };
        let c = cf_file_to_candidate("12345", "Test", f).unwrap();
        assert_eq!(c.mc_versions, vec!["1.21.5", "1.21.4"]);
        assert_eq!(c.loaders, vec!["fabric"]);
        assert_eq!(c.file_sha1.as_deref(), Some("sha1abc"));
    }

    #[test]
    fn t_live_07_cf_file_without_download_url_returns_none() {
        let f = CfFile {
            id: 5000,
            mod_id: 12345,
            display_name: "Test".to_string(),
            file_name: "test.jar".to_string(),
            file_length: 2048,
            download_url: None,
            game_versions: vec!["1.21.5".to_string()],
            dependencies: vec![],
            hashes: vec![],
            release_type: 1,
        };
        assert!(cf_file_to_candidate("12345", "Test", f).is_none());
    }

    #[test]
    fn t_live_08_cf_dependency_relation_types_mapped() {
        let f = CfFile {
            id: 5000,
            mod_id: 12345,
            display_name: "Test".to_string(),
            file_name: "test.jar".to_string(),
            file_length: 2048,
            download_url: Some("https://x.com/t.jar".to_string()),
            game_versions: vec!["1.21.5".to_string()],
            dependencies: vec![
                crate::curseforge::api::CfDependency {
                    mod_id: 100,
                    relation_type: 3,
                },
                crate::curseforge::api::CfDependency {
                    mod_id: 200,
                    relation_type: 5,
                },
                crate::curseforge::api::CfDependency {
                    mod_id: 300,
                    relation_type: 2,
                },
            ],
            hashes: vec![],
            release_type: 1,
        };
        let c = cf_file_to_candidate("12345", "Test", f).unwrap();
        assert_eq!(c.dependencies.len(), 3);
        let kinds: Vec<_> = c.dependencies.iter().map(|d| d.kind).collect();
        assert!(kinds.contains(&DependencyKind::Required));
        assert!(kinds.contains(&DependencyKind::Incompatible));
        assert!(kinds.contains(&DependencyKind::Optional));
    }
}
