use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeColors {
    pub bg_dark: [u8; 3],
    pub bg_panel: [u8; 3],
    pub bg_main: [u8; 3],
    pub bg_elevated: [u8; 3],
    pub bg_widget: [u8; 3],
    pub bg_widget_hover: [u8; 3],
    pub accent: [u8; 3],
    pub accent_light: [u8; 3],
    #[serde(default = "default_success")]
    pub success: [u8; 3],
    #[serde(default = "default_danger")]
    pub danger: [u8; 3],
    #[serde(default = "default_warning")]
    pub warning: [u8; 3],
    pub text_primary: [u8; 3],
    pub text_secondary: [u8; 3],
    pub text_muted: [u8; 3],
    #[serde(default = "default_text_disabled")]
    pub text_disabled: [u8; 3],
}

fn default_success() -> [u8; 3] {
    [110, 168, 120]
}
fn default_danger() -> [u8; 3] {
    [185, 105, 100]
}
fn default_warning() -> [u8; 3] {
    [200, 170, 95]
}
fn default_text_disabled() -> [u8; 3] {
    [90, 90, 90]
}

#[derive(Debug, Clone)]
pub struct LoadedTheme {
    pub file_name: String,
    pub theme: CustomTheme,
}

pub fn load_themes_from_dir(dir: &Path) -> Vec<LoadedTheme> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut themes = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let file_name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if file_name.starts_with('_') {
                continue;
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                match toml::from_str::<CustomTheme>(&content) {
                    Ok(theme) => themes.push(LoadedTheme { file_name, theme }),
                    Err(e) => {
                        tracing::warn!("Failed to parse theme {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
    themes
}

pub fn generate_example_theme() -> String {
    let example = CustomTheme {
        name: "My Custom Theme".to_string(),
        author: Some("Your Name".to_string()),
        colors: ThemeColors {
            bg_dark: [24, 25, 30],
            bg_panel: [28, 30, 35],
            bg_main: [33, 35, 40],
            bg_elevated: [40, 43, 50],
            bg_widget: [48, 52, 60],
            bg_widget_hover: [62, 67, 78],
            accent: [108, 142, 180],
            accent_light: [148, 178, 210],
            success: [110, 168, 120],
            danger: [185, 105, 100],
            warning: [200, 170, 95],
            text_primary: [218, 220, 228],
            text_secondary: [155, 160, 172],
            text_muted: [115, 120, 132],
            text_disabled: [78, 82, 92],
        },
    };
    toml::to_string_pretty(&example).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_theme_roundtrips() {
        let toml_str = generate_example_theme();
        let parsed: CustomTheme = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.name, "My Custom Theme");
        assert_eq!(parsed.colors.accent, [108, 142, 180]);
    }

    #[test]
    fn minimal_theme_parses() {
        let toml_str = r#"
name = "Minimal"

[colors]
bg_dark = [20, 20, 25]
bg_panel = [25, 25, 30]
bg_main = [30, 30, 35]
bg_elevated = [38, 38, 45]
bg_widget = [45, 45, 55]
bg_widget_hover = [58, 58, 70]
accent = [100, 130, 170]
accent_light = [140, 170, 200]
text_primary = [210, 212, 220]
text_secondary = [150, 152, 165]
text_muted = [110, 112, 125]
"#;
        let parsed: CustomTheme = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.name, "Minimal");
        assert_eq!(parsed.colors.success, default_success());
    }
}
