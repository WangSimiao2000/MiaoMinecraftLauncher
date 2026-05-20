mod mock_http;

use miao_core::config::LauncherConfig;
use miao_core::modloader::{ModLoaderType, fetch_all_loader_versions, install_loader};
use miao_core::modloader::{fabric, forge, neoforge, quilt};

use mock_http::MockHttpClient;

const FABRIC_VERSIONS_JSON: &str = r#"[
    {"loader": {"version": "0.16.0", "stable": true}},
    {"loader": {"version": "0.15.11", "stable": true}},
    {"loader": {"version": "0.15.10", "stable": false}}
]"#;

const QUILT_VERSIONS_JSON: &str = r#"[
    {"loader": {"version": "0.26.4"}},
    {"loader": {"version": "0.26.3"}}
]"#;

const NEOFORGE_VERSIONS_JSON: &str = r#"{
    "versions": ["20.4.237", "20.4.236", "20.4.200", "20.3.50", "19.4.1"]
}"#;

const FORGE_PROMOS_JSON: &str = r#"{
    "promos": {
        "1.20.4-recommended": "49.0.30",
        "1.20.4-latest": "49.0.31",
        "1.20.1-recommended": "47.2.0"
    }
}"#;

const FABRIC_PROFILE_JSON: &str = r#"{
    "id": "fabric-loader-0.16.0-1.20.4",
    "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
    "libraries": [
        {"name": "net.fabricmc:fabric-loader:0.16.0", "url": "https://maven.fabricmc.net/"}
    ]
}"#;

const QUILT_PROFILE_JSON: &str = r#"{
    "id": "quilt-loader-0.26.4-1.20.4",
    "mainClass": "org.quiltmc.loader.impl.launch.knot.KnotClient",
    "libraries": [
        {"name": "org.quiltmc:quilt-loader:0.26.4", "url": "https://maven.quiltmc.org/repository/release/"}
    ]
}"#;

const NEOFORGE_PROFILE_JSON: &str = r#"{
    "id": "neoforge-20.4.237",
    "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
    "libraries": [
        {"name": "net.neoforged:neoforge:20.4.237", "downloads": {"artifact": {"path": "net/neoforged/neoforge/20.4.237/neoforge-20.4.237.jar", "url": "https://maven.neoforged.net/releases/net/neoforged/neoforge/20.4.237/neoforge-20.4.237.jar", "sha1": "abc123", "size": 5000}}}
    ]
}"#;

const FORGE_PROFILE_JSON: &str = r#"{
    "id": "forge-1.20.4-49.0.30",
    "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
    "libraries": [
        {"name": "net.minecraftforge:forge:1.20.4-49.0.30", "downloads": {"artifact": {"path": "net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30.jar", "url": "https://files.minecraftforge.net/maven/net/minecraftforge/forge/1.20.4-49.0.30/forge-1.20.4-49.0.30.jar", "sha1": "def456", "size": 10000}}}
    ]
}"#;

#[tokio::test]
async fn fetch_fabric_loader_versions() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", FABRIC_VERSIONS_JSON);

    let versions = fabric::fetch_loader_versions(&mock, "1.20.4")
        .await
        .unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].loader.version, "0.16.0");
    assert!(versions[0].loader.stable);
    assert!(!versions[2].loader.stable);
}

#[tokio::test]
async fn fetch_fabric_loader_versions_empty() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", "[]");

    let versions = fabric::fetch_loader_versions(&mock, "0.0.1").await.unwrap();
    assert!(versions.is_empty());
}

#[tokio::test]
async fn fetch_quilt_loader_versions() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.quiltmc.org", QUILT_VERSIONS_JSON);

    let versions = quilt::fetch_loader_versions(&mock, "1.20.4").await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].loader.version, "0.26.4");
}

#[tokio::test]
async fn fetch_neoforge_versions() {
    let mock = MockHttpClient::new();
    mock.mock_response("maven.neoforged.net", NEOFORGE_VERSIONS_JSON);

    let versions = neoforge::fetch_versions(&mock, "1.20.4").await.unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0], "20.4.200");
    assert_eq!(versions[1], "20.4.236");
    assert_eq!(versions[2], "20.4.237");
}

#[tokio::test]
async fn fetch_neoforge_versions_no_match() {
    let mock = MockHttpClient::new();
    mock.mock_response("maven.neoforged.net", NEOFORGE_VERSIONS_JSON);

    let versions = neoforge::fetch_versions(&mock, "1.99.0").await.unwrap();
    assert!(versions.is_empty());
}

#[tokio::test]
async fn fetch_forge_recommended() {
    let mock = MockHttpClient::new();
    mock.mock_response("minecraftforge.net", FORGE_PROMOS_JSON);

    let version = forge::fetch_recommended_version(&mock, "1.20.4")
        .await
        .unwrap();
    assert_eq!(version, Some("49.0.30".to_string()));
}

#[tokio::test]
async fn fetch_forge_recommended_fallback_to_latest() {
    let json = r#"{"promos": {"1.20.4-latest": "49.0.31"}}"#;
    let mock = MockHttpClient::new();
    mock.mock_response("minecraftforge.net", json);

    let version = forge::fetch_recommended_version(&mock, "1.20.4")
        .await
        .unwrap();
    assert_eq!(version, Some("49.0.31".to_string()));
}

#[tokio::test]
async fn fetch_forge_no_version() {
    let mock = MockHttpClient::new();
    mock.mock_response("minecraftforge.net", r#"{"promos": {}}"#);

    let version = forge::fetch_recommended_version(&mock, "1.20.4")
        .await
        .unwrap();
    assert_eq!(version, None);
}

#[tokio::test]
async fn fetch_all_loader_versions_all_available() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", FABRIC_VERSIONS_JSON);
    mock.mock_response("meta.quiltmc.org", QUILT_VERSIONS_JSON);
    mock.mock_response("maven.neoforged.net", NEOFORGE_VERSIONS_JSON);
    mock.mock_response("minecraftforge.net", FORGE_PROMOS_JSON);

    let versions = fetch_all_loader_versions(&mock, "1.20.4").await.unwrap();

    assert!(versions.contains_key(&ModLoaderType::Fabric));
    assert!(versions.contains_key(&ModLoaderType::Quilt));
    assert!(versions.contains_key(&ModLoaderType::NeoForge));
    assert!(versions.contains_key(&ModLoaderType::Forge));

    assert_eq!(versions[&ModLoaderType::Fabric].len(), 3);
    assert_eq!(versions[&ModLoaderType::Quilt].len(), 2);
    assert_eq!(versions[&ModLoaderType::NeoForge].len(), 3);
    assert_eq!(versions[&ModLoaderType::Forge].len(), 1);
}

#[tokio::test]
async fn fetch_all_loader_versions_partial_failure() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", FABRIC_VERSIONS_JSON);

    let versions = fetch_all_loader_versions(&mock, "1.20.4").await.unwrap();

    assert!(versions.contains_key(&ModLoaderType::Fabric));
    assert!(!versions.contains_key(&ModLoaderType::Quilt));
    assert!(!versions.contains_key(&ModLoaderType::NeoForge));
    assert!(!versions.contains_key(&ModLoaderType::Forge));
}

#[tokio::test]
async fn fetch_all_loader_versions_stable_flag() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", FABRIC_VERSIONS_JSON);
    mock.mock_response("meta.quiltmc.org", QUILT_VERSIONS_JSON);
    mock.mock_response("maven.neoforged.net", r#"{"versions": []}"#);
    mock.mock_response("minecraftforge.net", r#"{"promos": {}}"#);

    let versions = fetch_all_loader_versions(&mock, "1.20.4").await.unwrap();

    let fabric = &versions[&ModLoaderType::Fabric];
    assert!(fabric[0].stable);
    assert!(fabric[1].stable);
    assert!(!fabric[2].stable);

    let quilt = &versions[&ModLoaderType::Quilt];
    assert!(quilt[0].stable);
}

#[tokio::test]
async fn fetch_fabric_profile() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", FABRIC_PROFILE_JSON);

    let profile = fabric::fetch_profile(&mock, "1.20.4", "0.16.0")
        .await
        .unwrap();
    assert_eq!(profile.id, "fabric-loader-0.16.0-1.20.4");
    assert_eq!(profile.libraries.len(), 1);
}

#[tokio::test]
async fn fetch_quilt_profile() {
    let mock = MockHttpClient::new();
    mock.mock_response("meta.quiltmc.org", QUILT_PROFILE_JSON);

    let profile = quilt::fetch_profile(&mock, "1.20.4", "0.26.4")
        .await
        .unwrap();
    assert_eq!(profile.id, "quilt-loader-0.26.4-1.20.4");
    assert_eq!(profile.libraries.len(), 1);
}

#[tokio::test]
async fn fetch_neoforge_profile() {
    let mock = MockHttpClient::new();
    mock.mock_response("maven.neoforged.net", NEOFORGE_PROFILE_JSON);

    let profile = neoforge::fetch_profile(&mock, "20.4.237").await.unwrap();
    assert_eq!(profile.id, "neoforge-20.4.237");
    assert_eq!(profile.libraries.len(), 1);
}

#[tokio::test]
async fn fetch_forge_install_profile() {
    let mock = MockHttpClient::new();
    mock.mock_response("minecraftforge.net", FORGE_PROFILE_JSON);

    let profile = forge::fetch_install_profile(&mock, "1.20.4", "49.0.30")
        .await
        .unwrap();
    assert_eq!(profile.id, "forge-1.20.4-49.0.30");
    assert_eq!(profile.libraries.len(), 1);
}

#[tokio::test]
async fn modloader_type_from_index() {
    assert_eq!(ModLoaderType::from_index(0), Some(ModLoaderType::Fabric));
    assert_eq!(ModLoaderType::from_index(1), Some(ModLoaderType::Quilt));
    assert_eq!(ModLoaderType::from_index(2), Some(ModLoaderType::NeoForge));
    assert_eq!(ModLoaderType::from_index(3), Some(ModLoaderType::Forge));
    assert_eq!(ModLoaderType::from_index(4), None);
}

#[tokio::test]
async fn modloader_type_as_str() {
    assert_eq!(ModLoaderType::Fabric.as_str(), "fabric");
    assert_eq!(ModLoaderType::Quilt.as_str(), "quilt");
    assert_eq!(ModLoaderType::NeoForge.as_str(), "neoforge");
    assert_eq!(ModLoaderType::Forge.as_str(), "forge");
}

#[tokio::test]
async fn modloader_type_all_have_str_names() {
    for lt in &ModLoaderType::ALL {
        assert!(!lt.as_str().is_empty());
    }
    assert_eq!(ModLoaderType::from_index(99), None);
}

#[tokio::test]
async fn install_loader_fabric_fetches_profile() {
    let profile_json = r#"{
        "id": "fabric-loader-0.16.0-1.20.4",
        "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
        "libraries": []
    }"#;

    let mock = MockHttpClient::new();
    mock.mock_response("meta.fabricmc.net", profile_json);

    let config = LauncherConfig::default();
    let result = install_loader(&mock, &ModLoaderType::Fabric, "1.20.4", "0.16.0", &config).await;
    let loader_config = result.unwrap();
    assert_eq!(loader_config.loader_type, ModLoaderType::Fabric);
    assert_eq!(loader_config.version, "0.16.0");
}
