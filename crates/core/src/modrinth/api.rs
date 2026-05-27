use crate::error::Result;
use serde::Deserialize;

use crate::http::HttpClient;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub dependency_type: String,
}

pub async fn search_mods(
    http: &impl HttpClient,
    query: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
    limit: u32,
) -> Result<SearchResult> {
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

    http.get_json(&url).await
}

pub async fn get_project_versions(
    http: &impl HttpClient,
    project_id: &str,
    mc_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<ProjectVersion>> {
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

    http.get_json(&url).await
}

pub async fn download_mod_file(
    http: &impl HttpClient,
    file: &VersionFile,
    dest_dir: &std::path::Path,
) -> Result<std::path::PathBuf> {
    let dest = dest_dir.join(&file.filename);

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let bytes = http.get_bytes(&file.url).await?;
    std::fs::write(&dest, &bytes)?;

    Ok(dest)
}

#[derive(Debug, Clone)]
pub struct InstalledMod {
    pub name: String,
    pub filename: String,
    pub is_dependency: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub project_id: String,
    pub name: String,
    pub filename: String,
    pub file_size: u64,
    pub is_dependency: bool,
}

pub async fn resolve_dependencies(
    http: &impl HttpClient,
    project_id: &str,
    mc_version: &str,
    loader: &str,
) -> Result<Vec<ResolvedDep>> {
    let mut deps = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = vec![(project_id.to_string(), false)];

    while let Some((pid, is_dependency)) = queue.pop() {
        if visited.contains(&pid) {
            continue;
        }
        visited.insert(pid.clone());

        let versions = get_project_versions(http, &pid, Some(mc_version), Some(loader)).await?;
        let Some(version) = versions.first() else {
            continue;
        };

        let Some(file) = version
            .files
            .iter()
            .find(|f| f.primary)
            .or(version.files.first())
        else {
            continue;
        };

        deps.push(ResolvedDep {
            project_id: pid.clone(),
            name: version.name.clone(),
            filename: file.filename.clone(),
            file_size: file.size,
            is_dependency,
        });

        for dep in &version.dependencies {
            if dep.dependency_type == "required"
                && let Some(ref dep_project_id) = dep.project_id
            {
                queue.push((dep_project_id.clone(), true));
            }
        }
    }

    Ok(deps)
}

pub async fn install_mod_only(
    http: &impl HttpClient,
    project_id: &str,
    mc_version: &str,
    loader: &str,
    mods_dir: &std::path::Path,
) -> Result<Vec<InstalledMod>> {
    let versions = get_project_versions(http, project_id, Some(mc_version), Some(loader)).await?;
    let Some(version) = versions.first() else {
        return Ok(Vec::new());
    };
    let Some(file) = version
        .files
        .iter()
        .find(|f| f.primary)
        .or(version.files.first())
    else {
        return Ok(Vec::new());
    };
    let dest = mods_dir.join(&file.filename);
    if !dest.exists() {
        download_mod_file(http, file, mods_dir).await?;
    }
    Ok(vec![InstalledMod {
        name: version.name.clone(),
        filename: file.filename.clone(),
        is_dependency: false,
    }])
}

pub async fn install_mod_with_dependencies(
    http: &impl HttpClient,
    project_id: &str,
    mc_version: &str,
    loader: &str,
    mods_dir: &std::path::Path,
) -> Result<Vec<InstalledMod>> {
    let mut installed: Vec<InstalledMod> = Vec::new();
    let mut visited = std::collections::HashSet::new();
    let mut queue = vec![(project_id.to_string(), false)];

    while let Some((pid, is_dependency)) = queue.pop() {
        if visited.contains(&pid) {
            continue;
        }
        visited.insert(pid.clone());

        let versions = get_project_versions(http, &pid, Some(mc_version), Some(loader)).await?;
        let version = match versions.first() {
            Some(v) => v,
            None => continue,
        };

        let file = match version
            .files
            .iter()
            .find(|f| f.primary)
            .or(version.files.first())
        {
            Some(f) => f,
            None => continue,
        };

        let dest = mods_dir.join(&file.filename);
        if !dest.exists() {
            download_mod_file(http, file, mods_dir).await?;
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
                queue.push((dep_project_id.clone(), true));
            }
        }
    }

    Ok(installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MiaoError;
    use std::collections::HashMap;
    use std::sync::Mutex;

    struct MockHttp {
        json_responses: Mutex<HashMap<String, String>>,
        byte_responses: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl MockHttp {
        fn new() -> Self {
            Self {
                json_responses: Mutex::new(HashMap::new()),
                byte_responses: Mutex::new(HashMap::new()),
            }
        }

        fn on_json(&self, url_contains: &str, body: &str) {
            self.json_responses
                .lock()
                .unwrap()
                .insert(url_contains.to_string(), body.to_string());
        }

        fn on_bytes(&self, url_contains: &str, data: Vec<u8>) {
            self.byte_responses
                .lock()
                .unwrap()
                .insert(url_contains.to_string(), data);
        }
    }

    impl HttpClient for MockHttp {
        async fn get_json<T: serde::de::DeserializeOwned + Send>(&self, url: &str) -> Result<T> {
            let responses = self.json_responses.lock().unwrap();
            for (pattern, body) in responses.iter() {
                if url.contains(pattern) {
                    return serde_json::from_str(body)
                        .map_err(|e| MiaoError::Other(format!("Mock parse error: {}", e)));
                }
            }
            Err(MiaoError::Other(format!("No mock for URL: {}", url)))
        }

        async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
            let responses = self.byte_responses.lock().unwrap();
            for (pattern, data) in responses.iter() {
                if url.contains(pattern) {
                    return Ok(data.clone());
                }
            }
            Err(MiaoError::Other(format!("No byte mock for URL: {}", url)))
        }
    }

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

    #[tokio::test]
    async fn search_mods_builds_correct_url_and_returns_results() {
        let mock = MockHttp::new();
        mock.on_json(
            "/search",
            r#"{"hits":[{"project_id":"AANobbMI","slug":"sodium","title":"Sodium","description":"Fast","author":"jelly","downloads":999,"categories":["fabric"],"versions":["1.20.4"],"icon_url":null}],"total_hits":1}"#,
        );

        let result = search_mods(&mock, "sodium", Some("1.20.4"), Some("fabric"), 10)
            .await
            .unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].slug, "sodium");
    }

    #[tokio::test]
    async fn search_mods_no_filters() {
        let mock = MockHttp::new();
        mock.on_json("/search", r#"{"hits":[],"total_hits":0}"#);

        let result = search_mods(&mock, "nothing", None, None, 5).await.unwrap();
        assert_eq!(result.hits.len(), 0);
    }

    #[tokio::test]
    async fn get_project_versions_with_filters() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/sodium/version",
            r#"[{"id":"v1","project_id":"AANobbMI","name":"Sodium 0.5.8","version_number":"0.5.8","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/sodium.jar","filename":"sodium.jar","primary":true,"size":1024,"hashes":{"sha1":"aaa","sha512":"bbb"}}],"dependencies":[],"version_type":"release","downloads":100,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let versions = get_project_versions(&mock, "sodium", Some("1.20.4"), Some("fabric"))
            .await
            .unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].name, "Sodium 0.5.8");
    }

    #[tokio::test]
    async fn get_project_versions_no_filters() {
        let mock = MockHttp::new();
        mock.on_json("/project/test-mod/version", r#"[]"#);

        let versions = get_project_versions(&mock, "test-mod", None, None)
            .await
            .unwrap();
        assert!(versions.is_empty());
    }

    #[tokio::test]
    async fn download_mod_file_writes_to_disk() {
        let mock = MockHttp::new();
        mock.on_bytes("cdn.example.com/test.jar", b"fake jar content".to_vec());

        let tmp = tempfile::tempdir().unwrap();
        let file = VersionFile {
            url: "https://cdn.example.com/test.jar".to_string(),
            filename: "test-mod.jar".to_string(),
            primary: true,
            size: 16,
            hashes: FileHashes {
                sha1: Some("x".to_string()),
                sha512: Some("y".to_string()),
            },
        };

        let dest = download_mod_file(&mock, &file, tmp.path()).await.unwrap();
        assert_eq!(dest, tmp.path().join("test-mod.jar"));
        assert_eq!(std::fs::read(&dest).unwrap(), b"fake jar content");
    }

    #[tokio::test]
    async fn resolve_dependencies_single_mod_no_deps() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/sodium/version",
            r#"[{"id":"v1","project_id":"AANobbMI","name":"Sodium 0.5.8","version_number":"0.5.8","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/sodium.jar","filename":"sodium.jar","primary":true,"size":1024,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[],"version_type":"release","downloads":100,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let deps = resolve_dependencies(&mock, "sodium", "1.20.4", "fabric")
            .await
            .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].project_id, "sodium");
        assert!(!deps[0].is_dependency);
    }

    #[tokio::test]
    async fn resolve_dependencies_with_chain() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/mod-a/version",
            r#"[{"id":"v1","project_id":"mod-a","name":"Mod A","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/a.jar","filename":"a.jar","primary":true,"size":100,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[{"project_id":"mod-b","version_id":null,"dependency_type":"required"}],"version_type":"release","downloads":50,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );
        mock.on_json(
            "/project/mod-b/version",
            r#"[{"id":"v2","project_id":"mod-b","name":"Mod B","version_number":"2.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/b.jar","filename":"b.jar","primary":true,"size":200,"hashes":{"sha1":"c","sha512":"d"}}],"dependencies":[],"version_type":"release","downloads":30,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let deps = resolve_dependencies(&mock, "mod-a", "1.20.4", "fabric")
            .await
            .unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].project_id, "mod-a");
        assert!(!deps[0].is_dependency);
        assert_eq!(deps[1].project_id, "mod-b");
        assert!(deps[1].is_dependency);
    }

    #[tokio::test]
    async fn resolve_dependencies_cycle_detection() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/cycle-a/version",
            r#"[{"id":"v1","project_id":"cycle-a","name":"Cycle A","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://x.com/a.jar","filename":"a.jar","primary":true,"size":10,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[{"project_id":"cycle-b","version_id":null,"dependency_type":"required"}],"version_type":"release","downloads":1,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );
        mock.on_json(
            "/project/cycle-b/version",
            r#"[{"id":"v2","project_id":"cycle-b","name":"Cycle B","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://x.com/b.jar","filename":"b.jar","primary":true,"size":10,"hashes":{"sha1":"c","sha512":"d"}}],"dependencies":[{"project_id":"cycle-a","version_id":null,"dependency_type":"required"}],"version_type":"release","downloads":1,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let deps = resolve_dependencies(&mock, "cycle-a", "1.20.4", "fabric")
            .await
            .unwrap();
        assert_eq!(deps.len(), 2);
    }

    #[tokio::test]
    async fn install_mod_only_downloads_file() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/test-mod/version",
            r#"[{"id":"v1","project_id":"test-mod","name":"Test Mod","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/test.jar","filename":"test.jar","primary":true,"size":64,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[],"version_type":"release","downloads":10,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );
        mock.on_bytes("cdn.example.com/test.jar", b"mod data".to_vec());

        let tmp = tempfile::tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();

        let installed = install_mod_only(&mock, "test-mod", "1.20.4", "fabric", &mods_dir)
            .await
            .unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].filename, "test.jar");
        assert!(!installed[0].is_dependency);
        assert_eq!(
            std::fs::read(mods_dir.join("test.jar")).unwrap(),
            b"mod data"
        );
    }

    #[tokio::test]
    async fn install_mod_only_skips_existing() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/existing/version",
            r#"[{"id":"v1","project_id":"existing","name":"Existing","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/existing.jar","filename":"existing.jar","primary":true,"size":32,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[],"version_type":"release","downloads":5,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let tmp = tempfile::tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();
        std::fs::write(mods_dir.join("existing.jar"), b"old data").unwrap();

        let installed = install_mod_only(&mock, "existing", "1.20.4", "fabric", &mods_dir)
            .await
            .unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(
            std::fs::read(mods_dir.join("existing.jar")).unwrap(),
            b"old data"
        );
    }

    #[tokio::test]
    async fn install_mod_with_dependencies_full_chain() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/main-mod/version",
            r#"[{"id":"v1","project_id":"main-mod","name":"Main","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/main.jar","filename":"main.jar","primary":true,"size":100,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[{"project_id":"dep-mod","version_id":null,"dependency_type":"required"}],"version_type":"release","downloads":50,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );
        mock.on_json(
            "/project/dep-mod/version",
            r#"[{"id":"v2","project_id":"dep-mod","name":"Dep","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://cdn.example.com/dep.jar","filename":"dep.jar","primary":true,"size":50,"hashes":{"sha1":"c","sha512":"d"}}],"dependencies":[],"version_type":"release","downloads":20,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );
        mock.on_bytes("cdn.example.com/main.jar", b"main content".to_vec());
        mock.on_bytes("cdn.example.com/dep.jar", b"dep content".to_vec());

        let tmp = tempfile::tempdir().unwrap();
        let mods_dir = tmp.path().join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();

        let installed =
            install_mod_with_dependencies(&mock, "main-mod", "1.20.4", "fabric", &mods_dir)
                .await
                .unwrap();
        assert_eq!(installed.len(), 2);
        assert!(std::fs::read(mods_dir.join("main.jar")).unwrap() == b"main content");
        assert!(std::fs::read(mods_dir.join("dep.jar")).unwrap() == b"dep content");
    }

    #[tokio::test]
    async fn install_mod_only_empty_versions() {
        let mock = MockHttp::new();
        mock.on_json("/project/empty/version", r#"[]"#);

        let tmp = tempfile::tempdir().unwrap();
        let installed = install_mod_only(&mock, "empty", "1.20.4", "fabric", tmp.path())
            .await
            .unwrap();
        assert!(installed.is_empty());
    }

    #[tokio::test]
    async fn resolve_dependencies_no_versions_available() {
        let mock = MockHttp::new();
        mock.on_json("/project/ghost/version", r#"[]"#);

        let deps = resolve_dependencies(&mock, "ghost", "1.20.4", "fabric")
            .await
            .unwrap();
        assert!(deps.is_empty());
    }

    #[tokio::test]
    async fn resolve_dependencies_optional_deps_ignored() {
        let mock = MockHttp::new();
        mock.on_json(
            "/project/with-optional/version",
            r#"[{"id":"v1","project_id":"with-optional","name":"WithOpt","version_number":"1.0","game_versions":["1.20.4"],"loaders":["fabric"],"files":[{"url":"https://x.com/a.jar","filename":"a.jar","primary":true,"size":10,"hashes":{"sha1":"a","sha512":"b"}}],"dependencies":[{"project_id":"optional-dep","version_id":null,"dependency_type":"optional"}],"version_type":"release","downloads":1,"date_published":"2024-01-01T00:00:00Z"}]"#,
        );

        let deps = resolve_dependencies(&mock, "with-optional", "1.20.4", "fabric")
            .await
            .unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].project_id, "with-optional");
    }
}
