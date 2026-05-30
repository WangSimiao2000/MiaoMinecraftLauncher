mod mock_http;

use miao_core::config::DownloadMirror;
use miao_core::version::install::fetch_version_meta;

use mock_http::MockHttpClient;

const VERSION_META_JSON: &str = r#"{
    "id": "1.20.4",
    "mainClass": "net.minecraft.client.main.Main",
    "type": "release",
    "assets": "12",
    "assetIndex": {
        "id": "12",
        "sha1": "abc123",
        "size": 400000,
        "totalSize": 600000,
        "url": "https://piston-meta.mojang.com/v1/packages/abc123/12.json"
    },
    "downloads": {
        "client": {
            "sha1": "def456",
            "size": 25000000,
            "url": "https://piston-data.mojang.com/v1/objects/def456/client.jar"
        }
    },
    "javaVersion": {
        "majorVersion": 17
    },
    "libraries": [
        {
            "name": "com.mojang:patchy:2.2.10",
            "downloads": {
                "artifact": {
                    "path": "com/mojang/patchy/2.2.10/patchy-2.2.10.jar",
                    "sha1": "ghi789",
                    "size": 15000,
                    "url": "https://libraries.minecraft.net/com/mojang/patchy/2.2.10/patchy-2.2.10.jar"
                }
            }
        },
        {
            "name": "org.lwjgl:lwjgl:3.3.3",
            "downloads": {
                "artifact": {
                    "path": "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar",
                    "sha1": "jkl012",
                    "size": 800000,
                    "url": "https://libraries.minecraft.net/org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar"
                }
            },
            "rules": [{"action": "allow", "os": {"name": "linux"}}]
        },
        {
            "name": "ca.weblite:java-objc-bridge:1.1",
            "downloads": {
                "artifact": {
                    "path": "ca/weblite/java-objc-bridge/1.1/java-objc-bridge-1.1.jar",
                    "sha1": "mno345",
                    "size": 40000,
                    "url": "https://libraries.minecraft.net/ca/weblite/java-objc-bridge/1.1/java-objc-bridge-1.1.jar"
                }
            },
            "rules": [{"action": "allow", "os": {"name": "osx"}}]
        }
    ]
}"#;

#[tokio::test]
async fn fetch_version_meta_success() {
    let mock = MockHttpClient::new();
    mock.mock_response("piston-meta.mojang.com", VERSION_META_JSON);

    let meta = fetch_version_meta(
        &mock,
        "https://piston-meta.mojang.com/v1/packages/abc/1.20.4.json",
        &DownloadMirror::Official,
    )
    .await
    .unwrap();

    assert_eq!(meta.id, "1.20.4");
    assert_eq!(meta.main_class, "net.minecraft.client.main.Main");
    assert_eq!(meta.required_java_major(), 17);
}

#[tokio::test]
async fn fetch_version_meta_with_bmclapi_mirror() {
    let mock = MockHttpClient::new();
    mock.mock_response("bmclapi2.bangbang93.com", VERSION_META_JSON);

    let meta = fetch_version_meta(
        &mock,
        "https://piston-meta.mojang.com/v1/packages/abc/1.20.4.json",
        &DownloadMirror::Bmclapi,
    )
    .await
    .unwrap();

    assert_eq!(meta.id, "1.20.4");
}

#[tokio::test]
async fn fetch_version_meta_network_error() {
    let mock = MockHttpClient::new();
    let result = fetch_version_meta(
        &mock,
        "https://piston-meta.mojang.com/v1/packages/abc/1.20.4.json",
        &DownloadMirror::Official,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn fetch_version_meta_official_failure_falls_back_to_bmclapi() {
    let mock = MockHttpClient::new();
    mock.mock_response("bmclapi2.bangbang93.com", VERSION_META_JSON);

    let meta = fetch_version_meta(
        &mock,
        "https://piston-meta.mojang.com/v1/packages/abc/1.20.4.json",
        &DownloadMirror::Official,
    )
    .await
    .expect("should fall back to BMCLAPI when Mojang fails");
    assert_eq!(meta.id, "1.20.4");
}
