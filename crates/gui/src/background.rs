use std::path::Path;

use eframe::egui;

pub struct BackgroundState {
    texture: Option<egui::TextureHandle>,
    loaded_path: Option<String>,
}

impl BackgroundState {
    pub fn new() -> Self {
        Self {
            texture: None,
            loaded_path: None,
        }
    }

    pub fn ensure_loaded(&mut self, ctx: &egui::Context, path: Option<&str>) {
        let target = path.map(|s| s.to_string());
        if self.loaded_path == target {
            return;
        }

        self.loaded_path = target.clone();
        self.texture = None;

        let Some(path_str) = target else {
            return;
        };

        let img_path = Path::new(&path_str);
        if !img_path.exists() {
            return;
        }

        let Ok(file_bytes) = std::fs::read(img_path) else {
            return;
        };

        let Ok(dyn_image) = image::load_from_memory(&file_bytes) else {
            return;
        };

        let rgba = dyn_image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let pixels = rgba.into_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
        let texture = ctx.load_texture("bg_custom", color_image, egui::TextureOptions::LINEAR);
        self.texture = Some(texture);
    }

    pub fn render(&self, ctx: &egui::Context) {
        let Some(ref texture) = self.texture else {
            return;
        };

        let screen = ctx.screen_rect();
        let tex_size = texture.size_vec2();

        let scale = (screen.width() / tex_size.x).max(screen.height() / tex_size.y);
        let scaled = tex_size * scale;
        let offset = (egui::vec2(screen.width(), screen.height()) - scaled) * 0.5;

        let paint_rect = egui::Rect::from_min_size(screen.min + offset, scaled);

        let area = egui::Area::new(egui::Id::new("bg_layer"))
            .fixed_pos(screen.min)
            .interactable(false)
            .order(egui::Order::Background);

        area.show(ctx, |ui| {
            ui.set_clip_rect(screen);
            ui.painter().image(
                texture.id(),
                paint_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            ui.painter().rect_filled(
                screen,
                egui::Rounding::ZERO,
                egui::Color32::from_black_alpha(140),
            );
        });
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        self.texture.is_some()
    }
}
