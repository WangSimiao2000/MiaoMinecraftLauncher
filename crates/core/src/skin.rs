use std::path::{Path, PathBuf};

use base64::Engine;
use image::{RgbaImage, imageops};
use serde::Deserialize;

const SESSION_SERVER: &str = "https://sessionserver.mojang.com/session/minecraft/profile";

#[derive(Debug, Deserialize)]
struct SessionProfile {
    properties: Vec<ProfileProperty>,
}

#[derive(Debug, Deserialize)]
struct ProfileProperty {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TexturePayload {
    textures: TextureMap,
}

#[derive(Debug, Deserialize)]
struct TextureMap {
    #[serde(rename = "SKIN")]
    skin: Option<TextureEntry>,
    #[serde(rename = "CAPE")]
    cape: Option<TextureEntry>,
}

#[derive(Debug, Deserialize)]
struct TextureEntry {
    url: String,
}

pub struct SkinCache {
    cache_dir: PathBuf,
}

impl SkinCache {
    pub fn new(data_dir: &Path) -> Self {
        let cache_dir = data_dir.join("skin_cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        Self { cache_dir }
    }

    pub fn avatar_path(&self, uuid: &str) -> PathBuf {
        self.cache_dir.join(format!("{}_head.png", uuid))
    }

    pub fn cape_path(&self, uuid: &str) -> PathBuf {
        self.cache_dir.join(format!("{}_cape.png", uuid))
    }

    pub fn has_avatar(&self, uuid: &str) -> bool {
        self.avatar_path(uuid).exists()
    }

    pub fn has_cape(&self, uuid: &str) -> bool {
        self.cape_path(uuid).exists()
    }
}

pub async fn fetch_and_cache_textures(
    http: &reqwest::Client,
    uuid: &str,
    cache: &SkinCache,
) -> anyhow::Result<()> {
    let uuid_clean = uuid.replace('-', "");
    let url = format!("{}/{}", SESSION_SERVER, uuid_clean);

    let resp = http.get(&url).send().await?.error_for_status()?;
    let profile: SessionProfile = resp.json().await?;

    let textures_b64 = profile
        .properties
        .iter()
        .find(|p| p.name == "textures")
        .map(|p| &p.value)
        .ok_or_else(|| anyhow::anyhow!("No textures property in profile"))?;

    let decoded = base64::engine::general_purpose::STANDARD.decode(textures_b64)?;
    let payload: TexturePayload = serde_json::from_slice(&decoded)?;

    if let Some(skin) = &payload.textures.skin {
        let skin_bytes = http.get(&skin.url).send().await?.bytes().await?;
        if let Ok(skin_img) = image::load_from_memory(&skin_bytes) {
            let head = crop_head(&skin_img);
            let _ = head.save(cache.avatar_path(&uuid_clean));
        }
    }

    if let Some(cape) = &payload.textures.cape {
        let cape_bytes = http.get(&cape.url).send().await?.bytes().await?;
        if let Ok(cape_img) = image::load_from_memory(&cape_bytes) {
            let front = crop_cape_front(&cape_img);
            let _ = front.save(cache.cape_path(&uuid_clean));
        }
    }

    Ok(())
}

fn crop_head(skin: &image::DynamicImage) -> RgbaImage {
    let scale = skin.width() / 64;

    let face = skin.crop_imm(8 * scale, 8 * scale, 8 * scale, 8 * scale);
    let hat = skin.crop_imm(40 * scale, 8 * scale, 8 * scale, 8 * scale);

    let mut result = face.to_rgba8();
    imageops::overlay(&mut result, &hat.to_rgba8(), 0, 0);

    imageops::resize(&result, 64, 64, imageops::FilterType::Nearest)
}

fn crop_cape_front(cape: &image::DynamicImage) -> RgbaImage {
    let scale = cape.width() / 64;
    let front = cape.crop_imm(scale, scale, 10 * scale, 16 * scale);
    imageops::resize(&front.to_rgba8(), 40, 64, imageops::FilterType::Nearest)
}

pub fn default_head_for_uuid(uuid: &str) -> RgbaImage {
    let is_steve = is_steve_model(uuid);
    let mut img = RgbaImage::new(64, 64);
    let color = if is_steve {
        image::Rgba([198, 178, 150, 255])
    } else {
        image::Rgba([161, 130, 98, 255])
    };
    for pixel in img.pixels_mut() {
        *pixel = color;
    }
    img
}

fn is_steve_model(uuid: &str) -> bool {
    let uuid_clean = uuid.replace('-', "");
    if uuid_clean.len() != 32 {
        return true;
    }
    let most = u64::from_str_radix(&uuid_clean[..16], 16).unwrap_or(0);
    let least = u64::from_str_radix(&uuid_clean[16..], 16).unwrap_or(0);
    let xored = most ^ least;
    ((xored >> 32) as u32 ^ xored as u32).is_multiple_of(2)
}
