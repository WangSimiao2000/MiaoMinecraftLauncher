use std::path::Path;

use crate::error::{CurseForgeError, Result};
use serde::{Deserialize, Serialize};

const CF_API_BASE: &str = "https://api.curseforge.com/v1";
const MINECRAFT_GAME_ID: u32 = 432;
const MODS_CLASS_ID: u32 = 6;

pub struct CurseForgeClient {
    http: reqwest::Client,
    api_key: String,
}

impl CurseForgeClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("MiaoMinecraftLauncher/0.1.0")
                .build()
                .expect("failed to build HTTP client"),
            api_key: api_key.into(),
        }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .http
            .get(url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "application/json")
            .send()
            .await?
            .error_for_status()?;
        let data = resp.json().await?;
        Ok(data)
    }

    #[allow(dead_code)]
    async fn post_json<T: serde::de::DeserializeOwned, B: Serialize + Send>(
        &self,
        url: &str,
        body: &B,
    ) -> Result<T> {
        let resp = self
            .http
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("Accept", "application/json")
            .json(body)
            .send()
            .await?
            .error_for_status()?;
        let data = resp.json().await?;
        Ok(data)
    }

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self
            .http
            .get(url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .error_for_status()?;
        Ok(resp.bytes().await?.to_vec())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfResponse<T> {
    pub data: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfPaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: CfPagination,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfPagination {
    pub index: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "resultCount")]
    pub result_count: u32,
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfMod {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    #[serde(rename = "downloadCount")]
    pub download_count: u64,
    pub categories: Vec<CfCategory>,
    pub authors: Vec<CfAuthor>,
    #[serde(rename = "latestFilesIndexes")]
    pub latest_files_indexes: Vec<CfFileIndex>,
    pub logo: Option<CfLogo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfCategory {
    pub id: u32,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfAuthor {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfLogo {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfFileIndex {
    #[serde(rename = "gameVersion")]
    pub game_version: String,
    #[serde(rename = "fileId")]
    pub file_id: u32,
    #[serde(rename = "modLoader")]
    pub mod_loader: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfFile {
    pub id: u32,
    #[serde(rename = "modId")]
    pub mod_id: u32,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "fileLength")]
    pub file_length: u64,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    #[serde(rename = "gameVersions")]
    pub game_versions: Vec<String>,
    pub dependencies: Vec<CfDependency>,
    pub hashes: Vec<CfHash>,
    #[serde(rename = "releaseType")]
    pub release_type: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfDependency {
    #[serde(rename = "modId")]
    pub mod_id: u32,
    #[serde(rename = "relationType")]
    pub relation_type: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CfHash {
    pub value: String,
    pub algo: u32,
}

fn loader_to_cf_type(loader: &str) -> Option<u32> {
    match loader.to_lowercase().as_str() {
        "forge" => Some(1),
        "cauldron" => Some(2),
        "liteloader" => Some(3),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

// CurseForge relationType: 1=EmbeddedLibrary, 2=Optional, 3=Required, 4=Tool, 5=Incompatible, 6=Include
const REQUIRED_DEPENDENCY: u32 = 3;

impl CurseForgeClient {
    pub async fn search_mods(
        &self,
        query: &str,
        mc_version: Option<&str>,
        loader: Option<&str>,
        limit: u32,
    ) -> Result<CfSearchResult> {
        let mut params = vec![
            format!("gameId={}", MINECRAFT_GAME_ID),
            format!("classId={}", MODS_CLASS_ID),
            format!("searchFilter={}", urlencoding(query)),
            format!("pageSize={}", limit.min(50)),
            "sortField=2".to_string(),
            "sortOrder=desc".to_string(),
        ];

        if let Some(mc) = mc_version {
            params.push(format!("gameVersion={}", mc));
        }
        if let Some(l) = loader {
            if let Some(loader_type) = loader_to_cf_type(l) {
                params.push(format!("modLoaderType={}", loader_type));
            }
        }

        let url = format!("{}/mods/search?{}", CF_API_BASE, params.join("&"));
        let resp: CfPaginatedResponse<CfMod> = self.get_json(&url).await?;

        Ok(CfSearchResult {
            mods: resp.data,
            total_count: resp.pagination.total_count,
        })
    }

    pub async fn get_mod(&self, mod_id: u32) -> Result<CfMod> {
        let url = format!("{}/mods/{}", CF_API_BASE, mod_id);
        let resp: CfResponse<CfMod> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn get_mod_files(
        &self,
        mod_id: u32,
        mc_version: Option<&str>,
        loader: Option<&str>,
    ) -> Result<Vec<CfFile>> {
        let mut params = vec![format!("gameId={}", MINECRAFT_GAME_ID)];

        if let Some(mc) = mc_version {
            params.push(format!("gameVersion={}", mc));
        }
        if let Some(l) = loader {
            if let Some(loader_type) = loader_to_cf_type(l) {
                params.push(format!("modLoaderType={}", loader_type));
            }
        }

        let url = format!("{}/mods/{}/files?{}", CF_API_BASE, mod_id, params.join("&"));
        let resp: CfPaginatedResponse<CfFile> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn get_mod_file(&self, mod_id: u32, file_id: u32) -> Result<CfFile> {
        let url = format!("{}/mods/{}/files/{}", CF_API_BASE, mod_id, file_id);
        let resp: CfResponse<CfFile> = self.get_json(&url).await?;
        Ok(resp.data)
    }

    pub async fn download_mod_file(
        &self,
        file: &CfFile,
        dest_dir: &Path,
    ) -> Result<std::path::PathBuf> {
        let download_url = file.download_url.as_deref().ok_or_else(|| {
            CurseForgeError::DownloadFailed(format!(
                "No download URL for file {} (mod may require manual download from website)",
                file.file_name
            ))
        })?;

        let dest = dest_dir.join(&file.file_name);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let bytes = self.get_bytes(download_url).await?;
        std::fs::write(&dest, &bytes)?;
        Ok(dest)
    }

    pub async fn resolve_dependencies(
        &self,
        mod_id: u32,
        mc_version: &str,
        loader: &str,
    ) -> Result<Vec<CfResolvedDep>> {
        let mut deps = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![(mod_id, false)];

        while let Some((current_mod_id, is_dependency)) = queue.pop() {
            if visited.contains(&current_mod_id) {
                continue;
            }
            visited.insert(current_mod_id);

            let files = self
                .get_mod_files(current_mod_id, Some(mc_version), Some(loader))
                .await?;

            let Some(file) = files.first() else {
                continue;
            };

            let mod_info = self.get_mod(current_mod_id).await?;

            deps.push(CfResolvedDep {
                mod_id: current_mod_id,
                name: mod_info.name,
                file_name: file.file_name.clone(),
                file_size: file.file_length,
                is_dependency,
            });

            for dep in &file.dependencies {
                if dep.relation_type == REQUIRED_DEPENDENCY {
                    queue.push((dep.mod_id, true));
                }
            }
        }

        Ok(deps)
    }

    pub async fn install_mod(
        &self,
        mod_id: u32,
        mc_version: &str,
        loader: &str,
        mods_dir: &Path,
    ) -> Result<Vec<CfInstalledMod>> {
        let mut installed = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![(mod_id, false)];

        while let Some((current_mod_id, is_dependency)) = queue.pop() {
            if visited.contains(&current_mod_id) {
                continue;
            }
            visited.insert(current_mod_id);

            let files = self
                .get_mod_files(current_mod_id, Some(mc_version), Some(loader))
                .await?;

            let Some(file) = files.first() else {
                continue;
            };

            let dest = mods_dir.join(&file.file_name);
            if !dest.exists() {
                self.download_mod_file(file, mods_dir).await?;
            }

            let mod_info = self.get_mod(current_mod_id).await?;

            installed.push(CfInstalledMod {
                name: mod_info.name,
                filename: file.file_name.clone(),
                is_dependency,
            });

            for dep in &file.dependencies {
                if dep.relation_type == REQUIRED_DEPENDENCY {
                    queue.push((dep.mod_id, true));
                }
            }
        }

        Ok(installed)
    }
}

#[derive(Debug, Clone)]
pub struct CfSearchResult {
    pub mods: Vec<CfMod>,
    pub total_count: u32,
}

#[derive(Debug, Clone)]
pub struct CfResolvedDep {
    pub mod_id: u32,
    pub name: String,
    pub file_name: String,
    pub file_size: u64,
    pub is_dependency: bool,
}

#[derive(Debug, Clone)]
pub struct CfInstalledMod {
    pub name: String,
    pub filename: String,
    pub is_dependency: bool,
}

fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_mapping() {
        assert_eq!(loader_to_cf_type("forge"), Some(1));
        assert_eq!(loader_to_cf_type("Fabric"), Some(4));
        assert_eq!(loader_to_cf_type("quilt"), Some(5));
        assert_eq!(loader_to_cf_type("neoforge"), Some(6));
        assert_eq!(loader_to_cf_type("unknown"), None);
    }

    #[test]
    fn cf_mod_deserializes() {
        let json = r#"{
            "id": 238222,
            "name": "Just Enough Items (JEI)",
            "slug": "jei",
            "summary": "View Items and Recipes",
            "downloadCount": 200000000,
            "categories": [{"id": 423, "name": "Map and Information", "slug": "map-information"}],
            "authors": [{"id": 123, "name": "mezz"}],
            "latestFilesIndexes": [{"gameVersion": "1.20.4", "fileId": 5000000, "modLoader": 1}],
            "logo": {"url": "https://media.forgecdn.net/avatars/jei.png"}
        }"#;
        let m: CfMod = serde_json::from_str(json).unwrap();
        assert_eq!(m.id, 238222);
        assert_eq!(m.name, "Just Enough Items (JEI)");
        assert_eq!(m.slug, "jei");
        assert_eq!(m.download_count, 200000000);
    }

    #[test]
    fn cf_file_deserializes() {
        let json = r#"{
            "id": 5000001,
            "modId": 238222,
            "displayName": "jei-1.20.4-forge-17.0.0.0.jar",
            "fileName": "jei-1.20.4-forge-17.0.0.0.jar",
            "fileLength": 1048576,
            "downloadUrl": "https://edge.forgecdn.net/files/5000/1/jei-1.20.4-forge-17.0.0.0.jar",
            "gameVersions": ["1.20.4", "Forge"],
            "dependencies": [{"modId": 300, "relationType": 3}],
            "hashes": [{"value": "abc123def456", "algo": 1}],
            "releaseType": 1
        }"#;
        let f: CfFile = serde_json::from_str(json).unwrap();
        assert_eq!(f.id, 5000001);
        assert_eq!(f.mod_id, 238222);
        assert_eq!(f.file_length, 1048576);
        assert!(f.download_url.is_some());
        assert_eq!(f.dependencies.len(), 1);
        assert_eq!(f.dependencies[0].relation_type, REQUIRED_DEPENDENCY);
    }

    #[test]
    fn cf_file_no_download_url() {
        let json = r#"{
            "id": 5000002,
            "modId": 12345,
            "displayName": "restricted-mod.jar",
            "fileName": "restricted-mod.jar",
            "fileLength": 512000,
            "downloadUrl": null,
            "gameVersions": ["1.20.4"],
            "dependencies": [],
            "hashes": [],
            "releaseType": 1
        }"#;
        let f: CfFile = serde_json::from_str(json).unwrap();
        assert!(f.download_url.is_none());
    }

    #[test]
    fn urlencoding_spaces_and_special() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("just enough items"), "just+enough+items");
    }

    #[test]
    fn cf_paginated_response_deserializes() {
        let json = r#"{
            "data": [],
            "pagination": {
                "index": 0,
                "pageSize": 20,
                "resultCount": 0,
                "totalCount": 0
            }
        }"#;
        let resp: CfPaginatedResponse<CfMod> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.pagination.page_size, 20);
        assert_eq!(resp.pagination.total_count, 0);
    }
}
