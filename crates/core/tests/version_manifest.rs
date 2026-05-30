mod mock_http;

use miao_core::config::DownloadMirror;
use miao_core::version::manifest::fetch_version_manifest;

use mock_http::MockHttpClient;

const MANIFEST_JSON: &str = r#"{
    "versions": [
        {"id": "1.20.4", "type": "release", "url": "https://example.com/1.20.4.json", "releaseTime": "2024-01-01"},
        {"id": "24w01a", "type": "snapshot", "url": "https://example.com/24w01a.json", "releaseTime": "2024-01-02"},
        {"id": "1.20.3", "type": "release", "url": "https://example.com/1.20.3.json", "releaseTime": "2023-12-01"},
        {"id": "b1.8.1", "type": "old_beta", "url": "https://example.com/b1.8.1.json", "releaseTime": "2011-09-19"},
        {"id": "a1.0.0", "type": "old_alpha", "url": "https://example.com/a1.0.0.json", "releaseTime": "2010-06-30"}
    ]
}"#;

#[tokio::test]
async fn fetch_manifest_official_mirror() {
    let mock = MockHttpClient::new();
    mock.mock_response("piston-meta.mojang.com", MANIFEST_JSON);

    let versions = fetch_version_manifest(&mock, &DownloadMirror::Official)
        .await
        .unwrap();
    assert_eq!(versions.len(), 5);
    assert_eq!(versions[0].id, "1.20.4");
    assert!(versions[0].is_release());
}

#[tokio::test]
async fn fetch_manifest_bmclapi_mirror() {
    let mock = MockHttpClient::new();
    mock.mock_response("bmclapi2.bangbang93.com", MANIFEST_JSON);

    let versions = fetch_version_manifest(&mock, &DownloadMirror::Bmclapi)
        .await
        .unwrap();
    assert_eq!(versions.len(), 5);
}

#[tokio::test]
async fn fetch_manifest_parses_version_types() {
    let mock = MockHttpClient::new();
    mock.mock_response("piston-meta.mojang.com", MANIFEST_JSON);

    let versions = fetch_version_manifest(&mock, &DownloadMirror::Official)
        .await
        .unwrap();

    assert!(versions[0].is_release());
    assert!(!versions[1].is_release());
    assert!(!versions[3].is_release());
    assert!(!versions[4].is_release());
}

#[tokio::test]
async fn fetch_manifest_network_error() {
    let mock = MockHttpClient::new();
    let result = fetch_version_manifest(&mock, &DownloadMirror::Official).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn fetch_manifest_official_failure_falls_back_to_bmclapi() {
    let mock = MockHttpClient::new();
    mock.mock_response("bmclapi2.bangbang93.com", MANIFEST_JSON);

    let versions = fetch_version_manifest(&mock, &DownloadMirror::Official)
        .await
        .expect("should fall back to BMCLAPI when Mojang fails");
    assert_eq!(versions.len(), 5);
    assert_eq!(versions[0].id, "1.20.4");
}

#[tokio::test]
async fn fetch_manifest_bmclapi_failure_falls_back_to_official() {
    let mock = MockHttpClient::new();
    mock.mock_response("piston-meta.mojang.com", MANIFEST_JSON);

    let versions = fetch_version_manifest(&mock, &DownloadMirror::Bmclapi)
        .await
        .expect("should fall back to Mojang when BMCLAPI fails");
    assert_eq!(versions.len(), 5);
}
