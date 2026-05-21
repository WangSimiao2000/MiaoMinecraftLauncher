use anyhow::Result;
use serde::Deserialize;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";
const USER_AGENT: &str =
    "MiaoMinecraftLauncher/0.1.0 (github.com/WangSimiao2000/MiaoMinecraftLauncher)";

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub total_hits: u32,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct ProjectVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<VersionFile>,
    pub dependencies: Vec<Dependency>,
    pub version_type: String,
    pub downloads: u64,
    pub date_published: String,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct FileHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: String,
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("failed to build HTTP client")
}

pub async fn search_mods(
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
    limit: u32,
) -> Result<SearchResult> {
    let http = build_client();

    let mut facets = vec!["[\"project_type:mod\"]".to_string()];
    if let Some(mc) = mc_version {
        facets.push(format!("[\"versions:{}\"]", mc));
    }
    if let Some(l) = loader {
        facets.push(format!("[\"categories:{}\"]", l));
    }
    let facets_str = format!("[{}]", facets.join(","));

    let url = format!(
        "{}/search?query={}&facets={}&limit={}",
        MODRINTH_API, query, facets_str, limit
    );

    let result: SearchResult = http.get(&url).send().await?.json().await?;
    Ok(result)
}

pub async fn get_project_versions(
    project_id: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<ProjectVersion>> {
    let http = build_client();

    let mut params = Vec::new();
    if let Some(mc) = mc_version {
        params.push(format!("game_versions=[\"{}\"]", mc));
    }
    if let Some(l) = loader {
        params.push(format!("loaders=[\"{}\"]", l));
    }

    let url = if params.is_empty() {
        format!("{}/project/{}/version", MODRINTH_API, project_id)
    } else {
        format!(
            "{}/project/{}/version?{}",
            MODRINTH_API,
            project_id,
            params.join("&")
        )
    };

    let versions: Vec<ProjectVersion> = http.get(&url).send().await?.json().await?;
    Ok(versions)
}

pub async fn download_mod_file(
    file: &VersionFile,
    dest_dir: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let http = build_client();
    let dest = dest_dir.join(&file.filename);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let response = http.get(&file.url).send().await?.error_for_status()?;
    let bytes = response.bytes().await?;
    std::fs::write(&dest, &bytes)?;

    Ok(dest)
}

#[derive(Debug, Clone)]
pub struct InstalledMod {
    pub name: String,
    pub filename: String,
    pub is_dependency: bool,
}

pub async fn install_mod_with_dependencies(
    project_id: &str,
    mc_version: &str,
    loader: &str,
    mods_dir: &std::path::Path,
) -> Result<Vec<InstalledMod>> {
    let mut installed: Vec<InstalledMod> = Vec::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    install_recursive(
        project_id,
        mc_version,
        loader,
        mods_dir,
        &mut installed,
        &mut visited,
        false,
    )
    .await?;
    Ok(installed)
}

#[async_recursion::async_recursion]
async fn install_recursive(
    project_id: &str,
    mc_version: &str,
    loader: &str,
    mods_dir: &std::path::Path,
    installed: &mut Vec<InstalledMod>,
    visited: &mut std::collections::HashSet<String>,
    is_dependency: bool,
) -> Result<()> {
    if visited.contains(project_id) {
        return Ok(());
    }
    visited.insert(project_id.to_string());

    let versions = get_project_versions(project_id, Some(mc_version), Some(loader)).await?;
    let version = match versions.first() {
        Some(v) => v,
        None => return Ok(()),
    };

    let file = match version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
    {
        Some(f) => f,
        None => return Ok(()),
    };

    let dest = mods_dir.join(&file.filename);
    if !dest.exists() {
        download_mod_file(file, mods_dir).await?;
    }

    installed.push(InstalledMod {
        name: version.name.clone(),
        filename: file.filename.clone(),
        is_dependency,
    });

    for dep in &version.dependencies {
        if dep.dependency_type == "required"
            && let Some(ref dep_project_id) = dep.project_id
        {
            install_recursive(
                dep_project_id,
                mc_version,
                loader,
                mods_dir,
                installed,
                visited,
                true,
            )
            .await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_result_deserializes() {
        let json = r#"{
            "hits": [{
                "project_id": "AANobbMI",
                "slug": "sodium",
                "title": "Sodium",
                "description": "Rendering optimization mod",
                "author": "jellysquid3",
                "downloads": 100000000,
                "categories": ["fabric", "optimization"],
                "versions": ["1.20.4"],
                "icon_url": null
            }],
            "total_hits": 1
        }"#;
        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].title, "Sodium");
        assert_eq!(result.total_hits, 1);
    }

    #[test]
    fn project_version_deserializes() {
        let json = r#"{
            "id": "4GyXKCLd",
            "project_id": "AANobbMI",
            "name": "Sodium 0.5.8",
            "version_number": "mc1.20.4-0.5.8",
            "game_versions": ["1.20.3", "1.20.4"],
            "loaders": ["fabric", "quilt"],
            "files": [{
                "url": "https://cdn.modrinth.com/data/test.jar",
                "filename": "sodium-fabric-0.5.8.jar",
                "primary": true,
                "size": 949085,
                "hashes": {"sha1": "abc123", "sha512": "def456"}
            }],
            "dependencies": [],
            "version_type": "release",
            "downloads": 3772095,
            "date_published": "2024-02-01T20:33:48Z"
        }"#;
        let ver: ProjectVersion = serde_json::from_str(json).unwrap();
        assert_eq!(ver.name, "Sodium 0.5.8");
        assert_eq!(ver.files.len(), 1);
        assert!(ver.files[0].primary);
    }
}
