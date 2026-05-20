use crate::config::DownloadMirror;

const BMCLAPI_BASE: &str = "https://bmclapi2.bangbang93.com";
const MOJANG_LIBRARIES: &str = "https://libraries.minecraft.net";
const MOJANG_RESOURCES: &str = "https://resources.download.minecraft.net";
const MOJANG_LAUNCHER_META: &str = "https://piston-meta.mojang.com";

pub fn transform_url(url: &str, mirror: &DownloadMirror) -> String {
    match mirror {
        DownloadMirror::Official => url.to_string(),
        DownloadMirror::Bmclapi => url
            .replace(MOJANG_LIBRARIES, &format!("{}/maven", BMCLAPI_BASE))
            .replace(
                MOJANG_RESOURCES,
                &format!("{}/assets", BMCLAPI_BASE),
            )
            .replace(MOJANG_LAUNCHER_META, BMCLAPI_BASE),
        DownloadMirror::Custom(base) => url
            .replace(MOJANG_LIBRARIES, &format!("{}/maven", base))
            .replace(MOJANG_RESOURCES, &format!("{}/assets", base))
            .replace(MOJANG_LAUNCHER_META, base),
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
        assert_eq!(
            result,
            "https://bmclapi2.bangbang93.com/assets/ab/abc123"
        );
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
}
