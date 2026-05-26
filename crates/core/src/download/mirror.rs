use crate::config::DownloadMirror;

const BMCLAPI_BASE: &str = "https://bmclapi2.bangbang93.com";
const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net";
const MOJANG_RESOURCES: &str = "https://resources.download.minecraft.net";
const MOJANG_LAUNCHER_META: &str = "https://piston-meta.mojang.com";

/// Transform a Mojang URL to use the specified mirror.
pub fn transform_url(url: &str, mirror: &DownloadMirror) -> String {
    match mirror {
        DownloadMirror::Official => url.to_string(),
        DownloadMirror::Bmclapi => url
            .replace(MOJANG_LIBRARIES, &format!("{}/maven", BMCLAPI_BASE))
            .replace(MOJANG_RESOURCES, &format!("{}/assets", BMCLAPI_BASE))
            .replace(MOJANG_LAUNCHER_META, BMCLAPI_BASE),
        DownloadMirror::Custom(base) => url
            .replace(MOJANG_LIBRARIES, &format!("{}/maven", base))
            .replace(MOJANG_RESOURCES, &format!("{}/assets", base))
            .replace(MOJANG_LAUNCHER_META, base),
    }
}

/// Build a fallback chain of mirrors to try in order.
/// The primary mirror is always first, followed by fallbacks.
pub fn build_fallback_chain(primary: &DownloadMirror) -> Vec<DownloadMirror> {
    match primary {
        DownloadMirror::Official => vec![DownloadMirror::Official, DownloadMirror::Bmclapi],
        DownloadMirror::Bmclapi => vec![DownloadMirror::Bmclapi, DownloadMirror::Official],
        DownloadMirror::Custom(url) => vec![
            DownloadMirror::Custom(url.clone()),
            DownloadMirror::Bmclapi,
            DownloadMirror::Official,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_official_is_identity() {
        let url = "https://libraries.minecraft.net/com/mojang/some.jar";
        assert_eq!(transform_url(url, &DownloadMirror::Official), url);
    }

    #[test]
    fn transform_bmclapi_libraries() {
        let url = "https://libraries.minecraft.net/com/mojang/some.jar";
        let result = transform_url(url, &DownloadMirror::Bmclapi);
        assert_eq!(
            result,
            "https://bmclapi2.bangbang93.com/maven/com/mojang/some.jar"
        );
    }

    #[test]
    fn transform_bmclapi_assets() {
        let url = "https://resources.download.minecraft.net/ab/abc123";
        let result = transform_url(url, &DownloadMirror::Bmclapi);
        assert_eq!(result, "https://bmclapi2.bangbang93.com/assets/ab/abc123");
    }

    #[test]
    fn transform_custom_mirror() {
        let url = "https://libraries.minecraft.net/com/mojang/some.jar";
        let mirror = DownloadMirror::Custom("https://my-mirror.example.com".to_string());
        let result = transform_url(url, &mirror);
        assert_eq!(
            result,
            "https://my-mirror.example.com/maven/com/mojang/some.jar"
        );
    }

    #[test]
    fn fallback_chain_official_primary() {
        let chain = build_fallback_chain(&DownloadMirror::Official);
        assert_eq!(chain.len(), 2);
        assert!(matches!(chain[0], DownloadMirror::Official));
        assert!(matches!(chain[1], DownloadMirror::Bmclapi));
    }

    #[test]
    fn fallback_chain_bmclapi_primary() {
        let chain = build_fallback_chain(&DownloadMirror::Bmclapi);
        assert_eq!(chain.len(), 2);
        assert!(matches!(chain[0], DownloadMirror::Bmclapi));
        assert!(matches!(chain[1], DownloadMirror::Official));
    }

    #[test]
    fn fallback_chain_custom_primary() {
        let mirror = DownloadMirror::Custom("https://custom.example.com".to_string());
        let chain = build_fallback_chain(&mirror);
        assert_eq!(chain.len(), 3);
        assert!(
            matches!(&chain[0], DownloadMirror::Custom(u) if u == "https://custom.example.com")
        );
        assert!(matches!(chain[1], DownloadMirror::Bmclapi));
        assert!(matches!(chain[2], DownloadMirror::Official));
    }
}
